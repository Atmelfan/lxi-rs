use std::{io, sync::Arc, time::Duration};

use async_listen::ListenExt;
use async_std::{
    net::{TcpListener, ToSocketAddrs, UdpSocket},
    task,
};
use futures::{try_join, StreamExt};

use crate::common::{
    onc_rpc::prelude::*,
    portmapper::{
        xdr, PMAPPROC_DUMP, PMAPPROC_GETPORT, PMAPPROC_NULL, PORTMAPPER_PROG, PORTMAPPER_VERS,
    },
    xdr::prelude::*,
};

pub mod prelude {
    pub use super::StaticPortMap;
    pub use crate::common::portmapper::{
        xdr::Mapping, PORTMAPPER_PORT, PORTMAPPER_PROG, PORTMAPPER_PROT_TCP, PORTMAPPER_PROT_UDP,
        PORTMAPPER_VERS,
    };
}

/// Create a simple static portmapper
///
/// The static portmapper allows null, getport and dump procedures, others will respond with a ProcUnavail error
pub struct StaticPortMap<const N: usize> {
    mappings: [xdr::Mapping; N],
}

impl<const N: usize> StaticPortMap<N> {
    pub fn new(mappings: [xdr::Mapping; N]) -> Arc<Self> {
        Arc::new(Self { mappings })
    }

    /// Serve both TCP and UDP calls at standard address
    pub async fn bind(self: Arc<Self>, addrs: impl ToSocketAddrs + Clone) -> io::Result<()> {
        let a = {
            let socket = UdpSocket::bind(addrs.clone()).await?;
            self.clone().serve_udp(socket)
        };
        let b = {
            let listener = TcpListener::bind(addrs.clone()).await?;
            self.clone().serve_tcp(listener)
        };
        try_join!(a, b).map(|_| ())
    }

    /// Serve UDP calls
    pub async fn serve_udp(self: Arc<Self>, socket: UdpSocket) -> io::Result<()> {
        log::info!("Listening on UDP {:?}", socket.local_addr()?);
        self.serve_udp_socket(socket).await
    }

    /// Serve TCP calls
    pub async fn serve_tcp(self: Arc<Self>, listener: TcpListener) -> io::Result<()> {
        log::info!("Listening on TCP {}", listener.local_addr()?);
        let mut incoming = listener
            .incoming()
            .log_warnings(|warn| log::warn!("Listening error: {}", warn))
            .handle_errors(Duration::from_millis(100))
            .backpressure(10);

        while let Some((token, stream)) = incoming.next().await {
            let peer = stream.peer_addr()?;
            log::debug!("Accepted from: {}", peer);

            let s = self.clone();
            task::spawn(async move {
                if let Err(err) = s.serve_tcp_stream(stream).await {
                    log::debug!("Error processing client: {}", err)
                }
                drop(token);
            });
        }
        log::info!("Stopped");
        Ok(())
    }
}

#[async_trait::async_trait]
impl<const N: usize> RpcService for StaticPortMap<N> {
    async fn call(
        self: Arc<Self>,
        prog: u32,
        vers: u32,
        proc: u32,
        args: &mut io::Cursor<Vec<u8>>,
        ret: &mut io::Cursor<Vec<u8>>,
    ) -> Result<(), RpcError> {
        if prog != PORTMAPPER_PROG {
            return Err(RpcError::ProgUnavail);
        }
        if vers != PORTMAPPER_VERS {
            return Err(RpcError::ProgMissmatch(MissmatchInfo {
                low: PORTMAPPER_VERS,
                high: PORTMAPPER_VERS,
            }));
        }
        match proc {
            PMAPPROC_NULL => Ok(()),
            PMAPPROC_GETPORT => {
                let mut mapping = xdr::Mapping::default();
                mapping.read_xdr(args)?;
                let port = self
                    .mappings
                    .iter()
                    .find(|m| {
                        m.prog == mapping.prog && m.vers == mapping.vers && m.prot == mapping.prot
                    })
                    .map_or(0, |m| m.port as u16);
                port.write_xdr(ret)?;
                Ok(())
            }
            PMAPPROC_DUMP => {
                for mapping in self.mappings.iter() {
                    true.write_xdr(ret)?;
                    mapping.write_xdr(ret)?;
                }
                false.write_xdr(ret)?;
                Ok(())
            }
            _ => Err(RpcError::ProcUnavail),
        }
    }
}
