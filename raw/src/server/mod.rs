use std::fmt::Debug;
use std::time::Duration;

use async_std::path::Path;
use async_std::sync::Arc;
use async_std::task;
use futures::{lock::Mutex, AsyncReadExt};
use futures::{AsyncBufReadExt, AsyncWriteExt, StreamExt};

use async_std::io::{self, BufReader, Read, Write};
use async_std::net::{TcpListener, ToSocketAddrs};

use async_listen::ListenExt;

use lxi_device::lock::SpinMutex;
use lxi_device::{
    lock::{LockHandle, SharedLock},
    Device,
};

#[cfg(unix)]
use async_std::os::unix::net::UnixListener;

pub struct Server(ServerConfig);

impl Server {
    /// Listen to a socket for clients
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

            stream.set_nodelay(true)?;

            task::spawn(async move {
                let (reader, writer) = stream.split();
                if let Err(err) = s
                    .process_client(reader, writer, shared_lock, device, peer)
                    .await
                {
                    log::warn!("Error processing client: {}", err)
                }
                drop(token);
            });
        }
        Ok(())
    }

    /// Listen to a unix socket for client
    #[cfg(unix)]
    pub async fn accept_unix<DEV>(
        self: Arc<Self>,
        path: impl AsRef<Path>,
        shared_lock: Arc<SpinMutex<SharedLock>>,
        device: Arc<Mutex<DEV>>,
    ) -> io::Result<()>
    where
        DEV: Device + Send + 'static,
    {
        let listener = UnixListener::bind(path).await?;
        let local = listener.local_addr()?;
        let mut incoming = listener
            .incoming()
            .log_warnings(|warn| log::error!("{:?} listening error: {}", local, warn))
            .handle_errors(Duration::from_millis(100))
            .backpressure(self.0.limit);

        while let Some((token, stream)) = incoming.next().await {
            let s = self.clone();
            let peer = stream.peer_addr()?;
            log::info!("Accepted from: {:?}", peer);

            let shared_lock = shared_lock.clone();
            let device = device.clone();

            task::spawn(async move {
                let (reader, writer) = stream.split();
                if let Err(err) = s
                    .process_client(reader, writer, shared_lock, device, peer)
                    .await
                {
                    log::warn!("Error processing client: {}", err)
                }
                drop(token);
            });
        }
        Ok(())
    }

    /// Process a generic reader/writer
    pub async fn process_client<DEV, RD, WR, SA>(
        self: Arc<Self>,
        reader: RD,
        mut writer: WR,
        shared_lock: Arc<SpinMutex<SharedLock>>,
        device: Arc<Mutex<DEV>>,
        peer: SA,
    ) -> io::Result<()>
    where
        DEV: Device + Send,
        RD: Read + Unpin,
        WR: Write + Unpin,
        SA: Debug,
    {
        let mut reader = BufReader::with_capacity(self.0.read_buffer, reader);
        //let mut writer = BufWriter::with_capacity(self.0.write_buffer, writer);

        let mut cmd = Vec::with_capacity(self.0.read_buffer);

        let handle = LockHandle::new(shared_lock, device);

        loop {
            // Read a line from stream.
            let n = reader.read_until(self.0.read_termination, &mut cmd).await?;
            if n == 0 {
                log::info!("{:?} disconnected", peer);
                break;
            }

            log::trace!("{:?} read {} bytes", peer, cmd.len());

            let resp = {
                let mut device = handle.async_lock().await.unwrap();
                cmd.pop(); // Remove read_termination
                device.execute(&cmd)
            };

            // Write back
            if let Some(mut data) = resp {
                data.push(self.0.write_termination);
                log::trace!("{:?} write {} bytes", peer, data.len());
                writer.write_all(&data).await?;
                //writer.flush().await?;
            }

            // Clear until next message
            cmd.clear();
        }

        Ok(())
    }
}

/// Socket server configuration builder
///
pub struct ServerConfig {
    read_buffer: usize,
    limit: usize,
    read_termination: u8,
    write_termination: u8,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            read_buffer: 512 * 1024,
            limit: 10,
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

    /// Set the termination character for writes.
    ///
    pub fn backpressure(self, limit: usize) -> Self {
        Self { limit, ..self }
    }

    pub fn build(self) -> Arc<Server> {
        Arc::new(Server(self))
    }
}
