use std::pin::Pin;

use futures::io::{AsyncRead, AsyncWrite};

pub(crate) enum HislipStream<IO> {
    Insecure(IO),
    #[cfg(feature = "secure-capability")]
    Secure(async_rustls::server::TlsStream<IO>),
}

impl<IO> HislipStream<IO> {
    pub(crate) fn new(io: IO) -> Self {
        Self::Insecure(io)
    }

    pub(crate) fn is_secure(&self) -> bool {
        cfg_if::cfg_if!{
            if #[cfg(feature = "secure-capability")] {
                matches!(self, Self::Secure(..))
            } else {
                false
            }
        }
    }
}

impl<IO> HislipStream<IO>
where
    IO: AsyncRead + AsyncWrite + Unpin,
{
    #[cfg(feature = "secure-capability")]
    pub(crate) async fn start_tls(
        self,
        acceptor: &mut async_rustls::TlsAcceptor,
    ) -> Result<Self, (std::io::Error, Self)> {
        match self {
            HislipStream::Insecure(io) => {
                match acceptor.accept(io).into_failable().await {
                    // Success
                    Ok(tls) => Ok(Self::Secure(tls)),
                    // Failed to switch to TLS
                    Err((err, io)) => Err((err, Self::Insecure(io))),
                }
            },
            HislipStream::Secure(_) => Err((std::io::ErrorKind::Other.into(), self)),
        }
    }

    #[cfg(feature = "secure-capability")]
    pub(crate) async fn end_tls(self) -> Result<Self, (std::io::Error, Self)> {
        match self {
            HislipStream::Insecure(_) => Err((std::io::ErrorKind::Other.into(), self)),
            HislipStream::Secure(mut _tls) => {
                let (_io, _session) = _tls.get_mut();
                todo!("Implement end_tls when async-rustls is updated")
            }
        }
    }
}

impl<IO> AsyncRead for HislipStream<IO>
where
    IO: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        match self.get_mut() {
            HislipStream::Insecure(io) => Pin::new(io).poll_read(cx, buf),
            #[cfg(feature = "secure-capability")]
            HislipStream::Secure(tls) => Pin::new(tls).poll_read(cx, buf),
        }
    }
}

impl<IO> AsyncWrite for HislipStream<IO>
where
    IO: AsyncRead + AsyncWrite + Unpin,
{
    #[inline]
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        match self.get_mut() {
            HislipStream::Insecure(io) => Pin::new(io).poll_write(cx, buf),
            #[cfg(feature = "secure-capability")]
            HislipStream::Secure(tls) => Pin::new(tls).poll_write(cx, buf),
        }
    }

    #[inline]
    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            HislipStream::Insecure(io) => Pin::new(io).poll_flush(cx),
            #[cfg(feature = "secure-capability")]
            HislipStream::Secure(tls) => Pin::new(tls).poll_flush(cx),
        }
    }

    #[inline]
    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            HislipStream::Insecure(io) => Pin::new(io).poll_close(cx),
            #[cfg(feature = "secure-capability")]
            HislipStream::Secure(tls) => Pin::new(tls).poll_close(cx),
        }
    }
}
