use async_std::{
    io::{
        Read as AsyncRead,
        Write as AsyncWrite
    },
    net::TcpStream
};

pub struct ServerConfig {
    port: u16,
}

/// Socket server
pub struct Server<IO> {
    stream: IO
}

impl<IO> Server<IO> where IO: AsyncRead + AsyncWrite {


}

pub type TcpServer = Server<async_std::net::TcpStream>;

impl Server<async_std::net::TcpStream> {
    pub async fn listen_tcp(addr: impl ToSocketAddr) -> Self {

    }
}

#[cfg(unix)]
pub type UnixServer = Server<async_std::os::unix::net::UnixStream>;

#[cfg(feature = "tls")]
pub type TlsServer = Server<async_tls::server::TlsStream>;

