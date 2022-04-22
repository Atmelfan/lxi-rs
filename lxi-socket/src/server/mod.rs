use std::fmt::Debug;
use std::time::Duration;

use async_std::path::Path;
use async_std::sync::Arc;
use futures::{lock::Mutex, AsyncReadExt};
use futures::{AsyncBufReadExt, AsyncWriteExt, StreamExt};

use async_std::io::{self, BufReader, Read, Write};
use async_std::net::{TcpListener, ToSocketAddrs};

#[cfg(unix)]
use async_std::os::unix::net::UnixListener;

use async_std::task;

use async_listen::ListenExt;

use lxi_device::{
    lock::{LockHandle, SharedLock},
    Device,
};

mod util;

pub struct Server(ServerConfig);

impl Server {
    pub async fn serve<DEV>(
        self: Arc<Self>,
        addr: impl ToSocketAddrs,
        shared_lock: Arc<Mutex<SharedLock>>,
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
            log::info!("Accepted from: {}", peer);

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

    #[cfg(unix)]
    pub async fn serve_unix<DEV>(
        self: Arc<Self>,
        path: impl AsRef<Path>,
        shared_lock: Arc<Mutex<SharedLock>>,
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

    pub async fn process_client<DEV, RD, WR, SA>(
        self: Arc<Self>,
        reader: RD,
        mut writer: WR,
        shared_lock: Arc<Mutex<SharedLock>>,
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

        let mut cmd = Vec::new();

        let handle = LockHandle::new(shared_lock, device);

        loop {
            // Read a line from stream.
            let n = reader.read_until(self.0.read_termination, &mut cmd).await?;

            // If this is the end of stream, return.
            if n == 0 {
                log::info!("{:?} disconnected", peer);
                break;
            }

            if log::log_enabled!(log::Level::Debug) {
                log::debug!("{:?} read {:?}", peer, cmd);
            }

            'inner: loop {
                let mut resp = if let Ok(mut x) = handle.try_lock().await {
                    x.execute(&cmd)
                } else {
                    // Wait until lock becomes available
                    continue 'inner;
                };
                // Write back
                if !resp.is_empty() {
                    resp.push(self.0.write_termination);
                    if log::log_enabled!(log::Level::Debug) {
                        log::debug!("{:?} write {:?}", peer, resp);
                    }
                    writer.write_all(&resp).await?;
                    //writer.flush().await?;
                }
                break 'inner;
            }

            // Clear until next message
            cmd.clear();
        }

        Ok(())
    }
}

/// Socket server configuration builder
///
#[cfg_attr(feature = "serde", derive(Deserializer, Serializer))]
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
