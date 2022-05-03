use async_std::sync::{Arc, Mutex};
use async_std::{
    net::{TcpListener, TcpStream, ToSocketAddrs},
    task,
};

use crate::Result;
use futures::StreamExt;

#[derive(Debug, Copy, Clone)]
pub struct ServerConfig {}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {}
    }
}

pub struct Server {
    inner: Arc<Mutex<InnerServer>>,
    config: ServerConfig,
}

impl Server {
    pub fn new(_vendor_id: u16) -> Self {
        Server {
            inner: InnerServer::new(),
            config: ServerConfig::default(),
        }
    }

    /// Accept clients
    ///
    pub async fn accept(&self, addr: impl ToSocketAddrs) -> Result<()> {
        InnerServer::accept(self.inner.clone(), addr, self.config).await
    }
}

struct InnerServer {}

impl InnerServer {
    fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(InnerServer {}))
    }

    /// Start accepting connections from addr
    ///
    async fn accept(
        server: Arc<Mutex<InnerServer>>,
        addr: impl ToSocketAddrs,
        config: ServerConfig,
    ) -> Result<()> {
        let listener = TcpListener::bind(addr).await?;
        let mut incoming = listener.incoming();
        while let Some(stream) = incoming.next().await {
            let stream = stream?;
            let _handle = task::spawn(Self::handle_connection(server.clone(), stream, config));
        }
        Ok(())
    }

    /// The connection handling function.
    async fn handle_connection(
        _server: Arc<Mutex<InnerServer>>,
        tcp_stream: TcpStream,
        _config: ServerConfig,
    ) -> Result<()> {
        let peer_addr = tcp_stream.peer_addr()?;
        log::info!("{} connected", peer_addr);

        log::info!("{} disconnected", peer_addr);
        Ok(())
    }
}
