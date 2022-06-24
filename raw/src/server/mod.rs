use core::fmt::Display;

use alloc::sync::Arc;
use alloc::vec::Vec;

use futures::task::{LocalSpawn, LocalSpawnExt};
use lxi_device::lock::{Mutex, SpinMutex};
use lxi_device::{
    lock::{LockHandle, SharedLock},
    Device,
};

#[derive(Clone)]
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
        let mut buffer = [0u8; 1024];
        let mut cmd = Vec::with_capacity(self.0.read_buffer);

        let handle = LockHandle::new(shared_lock, device);

        loop {
            // Read a line from stream.
            let n = stack.receive(&mut socket, &mut buffer).await?;
            if n == 0 {
                log::info!("{} disconnected", peer);
                break;
            }
            cmd.extend_from_slice(&buffer[..n]);

            if let Some(nl) = cmd.iter().position(|c| *c == self.0.read_termination) {
                log::trace!("{} read {:?}", peer, &cmd[..=nl]);

                let resp = {
                    let mut device = handle.async_lock().await.unwrap();
                    device.execute(&cmd[..=nl])
                };

                log::trace!("{} write {:?}", peer, resp);

                // Write back if response is not empty
                if let Some(mut data) = resp {
                    // Add termination if response does not contain one
                    if data.last().map_or(true, |c| *c != self.0.write_termination) {
                        data.push(self.0.write_termination)
                    }

                    // Write until no more data remains
                    while !data.is_empty() {
                        let n = stack.send(&mut socket, &data).await?;
                        data.drain(..n);
                    }
                } 

                // Remove line from buffer
                cmd.drain(..=nl);
            }
        }

        Ok(())
    }
}

/// Socket server configuration builder
///
#[derive(Clone)]
pub struct ServerConfig {
    read_buffer: usize,
    read_termination: u8,
    write_termination: u8,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            read_buffer: 512 * 1024,
            read_termination: b'\n',
            write_termination: b'\n',
        }
    }
}

impl ServerConfig {
    pub fn new(read_buffer: usize, termination_char: u8) -> Self {
        ServerConfig {
            read_buffer,
            read_termination: termination_char,
            write_termination: termination_char,
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

    /// Set the termination character for reads.
    ///
    /// # Panics
    ///
    /// Panics if termination character is not a ASCII control code (e.g. LF, CR, etc).
    pub fn read_termination(self, read_termination: u8) -> Self {
        debug_assert!(read_termination.is_ascii_control());
        Self {
            read_termination,
            ..self
        }
    }

    /// Set the termination character for writes.
    ///
    /// # Panics
    ///
    /// Panics if termination character is not a ASCII control code (e.g. LF, CR, etc).
    pub fn write_termination(self, write_termination: u8) -> Self {
        debug_assert!(write_termination.is_ascii_control());
        Self {
            write_termination,
            ..self
        }
    }

    pub fn build(self) -> Arc<Server> {
        Arc::new(Server(self))
    }
}
