use std::{pin::Pin, sync::Arc};

use async_rustls::{server::TlsStream, TlsAcceptor, rustls::Session};
use async_std::{io, net::TcpStream};
use futures::{AsyncRead, AsyncWrite};

pub(crate) enum HislipStream {
    Open(TcpStream),
    Encrypted(TlsStream<TcpStream>),
}

impl HislipStream {
    pub async fn start_tls(self, acceptor: Arc<TlsAcceptor>) -> io::Result<Self> {
        match self {
            HislipStream::Open(stream) => acceptor
                .accept(stream)
                .await
                .map(|stream| Self::Encrypted(stream)),
            HislipStream::Encrypted(_) => Ok(self),
        }
    }

    pub async fn end_tls(mut self) -> io::Result<Self> {
        todo!()
    }
}

impl AsyncRead for HislipStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<io::Result<usize>> {
        match self.get_mut() {
            HislipStream::Open(stream) => Pin::new(stream).poll_read(cx, buf),
            HislipStream::Encrypted(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for HislipStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<io::Result<usize>> {
        match self.get_mut() {
            HislipStream::Open(stream) => Pin::new(stream).poll_write(cx, buf),
            HislipStream::Encrypted(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        match self.get_mut() {
            HislipStream::Open(stream) => Pin::new(stream).poll_flush(cx),
            HislipStream::Encrypted(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        match self.get_mut() {
            HislipStream::Open(stream) => Pin::new(stream).poll_close(cx),
            HislipStream::Encrypted(stream) => Pin::new(stream).poll_close(cx),
        }
    }
}
