use async_std::sync::{Arc, Mutex, RwLock};
use futures::AsyncReadExt;

use async_std::io::{self, BufReader, BufWriter};
use async_std::net::{TcpListener, TcpStream, ToSocketAddrs};
use async_std::prelude::*;
use async_std::task;

use lxi_device::{Device, LockHandle, SharedLock};

#[cfg(feature = "tls")]
use async_tls;

/// Socket server
#[cfg_attr(feature = "serde", derive(Deserializer, Serializer))]
pub struct TcpServerBuilder<A> {
    addr: A,
    write_buffer: usize,
    read_buffer: usize,
}

impl<A> TcpServerBuilder<A>
where
    A: ToSocketAddrs,
{
    pub fn new(addr: A) -> Self {
        TcpServerBuilder {
            addr,
            write_buffer: 512 * 1024,
            read_buffer: 512 * 1024,
        }
    }

    pub fn write_buffer(self, write_buffer: usize) -> Self {
        Self {
            write_buffer,
            ..self
        }
    }

    pub fn read_buffer(self, read_buffer: usize) -> Self {
        Self {
            read_buffer,
            ..self
        }
    }

    pub async fn serve(
        self,
        shared_lock: Arc<RwLock<SharedLock>>,
        device: Arc<Mutex<dyn Device + Send>>,
    ) -> io::Result<()> {
        let listener = TcpListener::bind(self.addr).await?;
        let mut incoming = listener.incoming();

        while let Some(stream) = incoming.next().await {
            // Communication channels
            let stream = stream?;
            log::info!("Accepted from: {}", stream.peer_addr()?);

            let shared_lock = shared_lock.clone();
            let device = device.clone();
            let rlen = self.read_buffer;
            let wlen = self.write_buffer;

            task::spawn(async move {
                Self::process_client(stream, shared_lock, device, rlen, wlen).await
            });
        }
        Ok(())
    }

    pub async fn serve_tls(
        self,
        shared_lock: Arc<RwLock<SharedLock>>,
        device: Arc<Mutex<dyn Device + Send>>,
    ) -> io::Result<()> {
        let listener = TlS;
        Ok(())
    }

    async fn process_client(
        stream: TcpStream,
        shared_lock: Arc<RwLock<SharedLock>>,
        device: Arc<Mutex<dyn Device + Send>>,
        rlen: usize,
        wlen: usize,
    ) -> io::Result<()> {
        let (reader, writer) = stream.split();
        let mut reader = BufReader::with_capacity(rlen, reader);
        let mut writer = BufWriter::with_capacity(wlen, writer);

        let mut cmd = Vec::new();

        let handle = LockHandle::new(shared_lock, device);

        loop {
            // Read a line from stream.
            let n = reader.read_until(b'\n', &mut cmd).await?;

            // If this is the end of stream, return.
            if n == 0 {
                break;
            }

            let resp = if let Some(mut x) = handle.try_lock().await {
                x.execute(&cmd)
            } else {
                break;
            };

            // Write back
            if !resp.is_empty() {
                writer.write_all(&resp).await?;
            }

            // Clear until next message
            cmd.clear();
        }

        Ok(())
    }
}
