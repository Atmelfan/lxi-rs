use async_std::{
    io::{
        Read as AsyncRead,
        Write as AsyncWrite
    },
    net::TcpStream
};

#[cfg(feature = "unix")]
use async_std::os::unix::net::UnixStream;

/// Socket client
pub struct Client<IO> {
    stream: IO
}

/// Tcp server
pub type TcpClient = Client<TcpStream>;

#[cfg(feature = "unix")]
pub type TlsClient = Client<UnixStream>;

#[cfg(feature = "tls")]
pub type TlsClient = Client<TlsStream>;


impl<IO> Client<IO> where IO: AsyncRead + AsyncWrite {


}
