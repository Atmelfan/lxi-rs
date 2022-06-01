use std::pin::Pin;

use async_std::{io, net::TcpStream};
use futures::{AsyncRead, AsyncWrite};

#[cfg(feature = "tls")]
use async_tls::{server::TlsStream, TlsAcceptor};

pub(crate) const HISLIP_TLS_BUSY: u8 = 0;
pub(crate) const HISLIP_TLS_SUCCESS: u8 = 1;
pub(crate) const HISLIP_TLS_ERROR: u8 = 3;

pub(crate) enum HislipStream<'a> {
    /// Unencrypted stream
    Open(&'a TcpStream),

    /// TLS encrypted stream
    #[cfg(feature = "tls")]
    Encrypted(TlsStream<&'a TcpStream>),
}

impl<'a> HislipStream<'a> {
    #[cfg(feature = "tls")]
    pub async fn start_tls(&mut self, acceptor: TlsAcceptor) -> io::Result<()> {
        match self {
            HislipStream::Open(stream) => {
                let e = acceptor.accept(*stream).await?;
                *self = HislipStream::Encrypted(e);
                Ok(())
            }
            HislipStream::Encrypted(_) => Ok(()),
        }
    }

    #[cfg(feature = "tls")]
    pub async fn end_tls(&mut self) -> io::Result<()> {
        Err(io::ErrorKind::Other.into())
    }
}

impl<'a> AsyncRead for HislipStream<'a> {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<io::Result<usize>> {
        match self.get_mut() {
            HislipStream::Open(stream) => Pin::new(stream).poll_read(cx, buf),
            #[cfg(feature = "tls")]
            HislipStream::Encrypted(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl<'a> AsyncWrite for HislipStream<'a> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<io::Result<usize>> {
        match self.get_mut() {
            HislipStream::Open(stream) => Pin::new(stream).poll_write(cx, buf),
            #[cfg(feature = "tls")]
            HislipStream::Encrypted(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        match self.get_mut() {
            HislipStream::Open(stream) => Pin::new(stream).poll_flush(cx),
            #[cfg(feature = "tls")]
            HislipStream::Encrypted(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        match self.get_mut() {
            HislipStream::Open(stream) => Pin::new(stream).poll_close(cx),
            #[cfg(feature = "tls")]
            HislipStream::Encrypted(stream) => Pin::new(stream).poll_close(cx),
        }
    }
}

#[cfg(test)]
mod tests {
    fn x() {}
}
