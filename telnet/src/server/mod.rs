use std::fmt::Debug;
use std::time::Duration;

use async_std::sync::Arc;
use async_std::task;
use futures::lock::Mutex;
use futures::{AsyncWriteExt, StreamExt};

use async_std::io::{self, Read, ReadExt, Write};
use async_std::net::{TcpListener, ToSocketAddrs};

use async_listen::ListenExt;

use libtelnet_rs::events::TelnetEvents;
use libtelnet_rs::{telnet::op_option as options, Parser};

use lxi_device::lock::SpinMutex;
use lxi_device::{
    lock::{LockHandle, SharedLock},
    Device,
};

pub struct Server(ServerConfig);

impl Server {
    /// Accept client connections
    pub async fn accept<DEV>(
        self: Arc<Self>,
        addr: impl ToSocketAddrs,
        shared_lock: Arc<SpinMutex<SharedLock>>,
        device: Arc<Mutex<DEV>>,
    ) -> io::Result<()>
    where
        DEV: Device + Send + 'static,
    {
        let listener = TcpListener::bind(addr).await?;
        let mut incoming = listener
            .incoming()
            .log_warnings(|warn| log::warn!("Listening error: {}", warn))
            .handle_errors(Duration::from_millis(100))
            .backpressure(self.0.limit);

        while let Some((token, stream)) = incoming.next().await {
            let s = self.clone();
            let peer = stream.peer_addr()?;
            log::error!("Accepted from: {}", peer);

            let shared_lock = shared_lock.clone();
            let device = device.clone();

            task::spawn(async move {
                if let Err(err) = s.process_client(stream, shared_lock, device, peer).await {
                    log::warn!("Error processing client: {}", err)
                }
                drop(token);
            });
        }
        Ok(())
    }

    /// Process a client stream
    pub async fn process_client<DEV, IO, SA>(
        self: Arc<Self>,
        mut stream: IO,
        shared_lock: Arc<SpinMutex<SharedLock>>,
        device: Arc<Mutex<DEV>>,
        _peer: SA,
    ) -> io::Result<()>
    where
        DEV: Device + Send,
        IO: Read + Write + Unpin,
        SA: Debug,
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
            stream.write_all(&prompt).await?;
            // Send a go ahead
            if !instance.options.get_option(options::SGA).local_state {
                stream.write_all(b"\xff\x03").await?;
            }

            // Read a line from stream.
            let n = stream.read(&mut buf).await?;

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
                                stream.write_all(&[b]).await?;
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
                                if let Some(data) = resp {
                                    let to_send = Parser::escape_iac(data);
                                    stream.write_all(&to_send).await?;
                                    stream.write_all(b"\r\n").await?;
                                }
                            } else {
                                cmd.push(b);
                            }
                        }
                    }
                    TelnetEvents::DataSend(data) => {
                        stream.write_all(&data).await?;
                    }
                    TelnetEvents::DecompressImmediate(_data) => unreachable!(),
                }
            }
        }
    }
}

/// Socket server configuration builder
///
#[cfg_attr(feature = "serde", derive(Deserializer, Serializer))]
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
