use core::fmt::{Debug, Display};
use core::time::Duration;

use alloc::sync::Arc;
use alloc::vec::Vec;
use futures::lock::Mutex;
use futures::task::{LocalSpawn, LocalSpawnExt};

use libtelnet_rs::events::TelnetEvents;
use libtelnet_rs::{telnet::op_option as options, Parser};

use lxi_device::lock::SpinMutex;
use lxi_device::{
    lock::{LockHandle, SharedLock},
    Device,
};

pub struct Server(ServerConfig);

impl Server {
    pub async fn bind<DEV, STACK, S>(
        self: Arc<Self>,
        mut stack: STACK,
        port: u16,
        shared_lock: Arc<SpinMutex<SharedLock>>,
        device: Arc<Mutex<DEV>>,
        spawner: S,
    ) -> Result<(), STACK::Error>
    where
        DEV: Device + 'static,
        STACK: embedded_nal_async::TcpFullStack + Clone + 'static,
        S: LocalSpawn,
    {
        let mut socket = stack.socket().await?;
        stack.bind(&mut socket, port).await?;
        self.accept(stack, socket, shared_lock, device, spawner)
            .await
    }

    pub async fn accept<DEV, STACK, S>(
        self: Arc<Self>,
        mut stack: STACK,
        mut socket: STACK::TcpSocket,
        shared_lock: Arc<SpinMutex<SharedLock>>,
        device: Arc<Mutex<DEV>>,
        spawner: S,
    ) -> Result<(), STACK::Error>
    where
        DEV: Device + 'static,
        STACK: embedded_nal_async::TcpFullStack + Clone + 'static,
        S: LocalSpawn,
    {
        loop {
            let (s, peer) = stack.accept(&mut socket).await?;
            log::info!("{peer} connected");

            let client_stack = stack.clone();
            let client_shared_lock = shared_lock.clone();
            let client_device = device.clone();
            let this = self.clone();
            spawner
                .spawn_local(async move {
                    let res = this
                        .process_client(client_stack, s, client_shared_lock, client_device, peer)
                        .await;
                    if let Err(err) = res {
                        log::error!("{peer} disconnected: {err:?}");
                    } else {
                        log::info!("{peer} disconnected")
                    }
                })
                .unwrap();
        }
    }

    pub async fn process_client<DEV, STACK>(
        &self,
        mut stack: STACK,
        mut socket: STACK::TcpSocket,
        shared_lock: Arc<SpinMutex<SharedLock>>,
        device: Arc<Mutex<DEV>>,
        peer: impl Display,
    ) -> Result<(), STACK::Error>
    where
        DEV: Device,
        STACK: embedded_nal_async::TcpFullStack,
    {
        let handle = LockHandle::new(shared_lock, device);

        let mut instance = Parser::new();
        instance.options.support_local(options::ECHO);
        instance.options.support_local(options::SGA);
        instance.options.support_local(options::BINARY);

        let mut buf = [0u8; 128];
        let mut cmd = Vec::with_capacity(self.0.read_buffer);

        let prompt = Parser::escape_iac(&b"SCPI> "[..]);

        loop {
            stack.send(&mut socket, &prompt).await?;
            // Send a go ahead
            if !instance.options.get_option(options::SGA).local_state {
                stack.send(&mut socket, b"\xff\x03").await?;
            }

            // Read a line from stream.
            let n = stack.receive(&mut socket, &mut buf).await?;

            let events = instance.receive(&buf[..n]);
            for event in events {
                match event {
                    TelnetEvents::IAC(iac) => match iac.command {
                        247 /* EC */ => {
                            cmd.pop();
                        },
                        248 /* EL */ => {
                            cmd.clear();
                        },
                        _cmd => {}
                    },
                    TelnetEvents::Negotiation(_neg) => {}
                    TelnetEvents::Subnegotiation(_sub) => {}
                    TelnetEvents::DataReceive(data) => {
                        for b in data {
                            // Echo back if enabled
                            if instance.options.get_option(options::ECHO).local_state {
                                stack.send(&mut socket, &[b]).await?;
                            }
                            
                            if b == b'\n' {
                                // Remove \r
                                cmd.pop();
                                // Lock device and execute
                                let resp = {
                                    let mut device = handle.async_lock().await.unwrap();
                                    device.execute(&cmd)
                                };
                                cmd.clear();

                                // Send back response if any
                                // Write back if response is not empty
                                if let Some(mut data) = resp {
                                    // Add termination if response does not contain one
                                    if !data.as_slice().ends_with(b"\r\n") {
                                        if data.ends_with(b"\n") {
                                            data.pop();
                                        }
                                        data.extend_from_slice(b"\r\n")
                                    }

                                    // Write until no more data remains
                                    while !data.is_empty() {
                                        let n = stack.send(&mut socket, &data).await?;
                                        data.drain(..n);
                                    }
                                } 
                            } else {
                                cmd.push(b);
                            }
                        }
                    }
                    TelnetEvents::DataSend(data) => {
                        stack.send(&mut socket, &data).await?;
                    }
                    TelnetEvents::DecompressImmediate(_data) => unreachable!(),
                }
            }
        }
    }

}

/// Socket server configuration builder
///
pub struct ServerConfig {
    read_buffer: usize,
    limit: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            read_buffer: 512 * 1024,
            limit: 10,
        }
    }
}

impl ServerConfig {
    pub fn new(read_buffer: usize) -> Self {
        ServerConfig {
            read_buffer,
            ..Default::default()
        }
    }

    /// Set the read buffer size
    ///
    pub fn read_buffer(self, read_buffer: usize) -> Self {
        Self {
            read_buffer,
            ..self
        }
    }

    /// Set the maximmum number of clients allowed to be served at once.
    ///
    pub fn backpressure(self, limit: usize) -> Self {
        Self { limit, ..self }
    }

    /// Finishes and reurns the server
    pub fn build(self) -> Arc<Server> {
        Arc::new(Server(self))
    }
}
