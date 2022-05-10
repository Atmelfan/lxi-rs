use std::{
    collections::HashMap,
    io::{self, Cursor, Error, Read, Write},
    net::IpAddr,
    sync::Arc,
    time::Duration,
};

use async_listen::ListenExt;
use async_std::{
    net::{TcpListener, ToSocketAddrs, UdpSocket},
    sync::RwLock,
    task,
};
use futures::{try_join, AsyncRead, AsyncWrite, StreamExt};

use crate::common::{
    onc_rpc::{RpcError, RpcService, StreamRpcClient},
    xdr::{
        basic::{XdrDecode, XdrEncode},
        onc_rpc::xdr::MissmatchInfo,
        portmapper::{
            PMAPPROC_CALLIT, PMAPPROC_DUMP, PMAPPROC_GETPORT, PMAPPROC_NULL, PMAPPROC_SET,
            PMAPPROC_UNSET, PORTMAPPER_PROG, PORTMAPPER_VERS,
        },
    },
};

pub mod prelude {
    pub use super::{PortMapperClient, StaticPortMap, StaticPortMapBuilder};
    pub use crate::common::xdr::portmapper::{
        xdr::Mapping, PORTMAPPER_PORT, PORTMAPPER_PROT_TCP, PORTMAPPER_PROT_UDP,
    }; 
}

use prelude::*;

pub struct PortMapperClient<IO>(StreamRpcClient<IO>);

impl<IO> PortMapperClient<IO>
where
    IO: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(io: IO) -> Self {
        Self(StreamRpcClient::new(io, PORTMAPPER_PROG, PORTMAPPER_VERS))
    }

    pub async fn null(&mut self, mapping: Mapping) -> Result<bool, RpcError> {
        self.0.call(PMAPPROC_NULL, mapping).await
    }

    pub async fn set(&mut self, mapping: Mapping) -> Result<bool, RpcError> {
        self.0.call(PMAPPROC_SET, mapping).await
    }

    pub async fn unset(&mut self, mapping: Mapping) -> Result<bool, RpcError> {
        self.0.call(PMAPPROC_UNSET, mapping).await
    }

    pub async fn getport(&mut self, mapping: Mapping) -> Result<u16, RpcError> {
        self.0.call(PMAPPROC_GETPORT, mapping).await
    }
}

#[cfg(test)]
mod test_portmap_client {
    use super::*;

    #[async_std::test]
    async fn test_call_rpc() {
        let stream = async_std::net::TcpStream::connect("127.0.0.1:111")
            .await
            .unwrap();
        println!("Connected to {}", &stream.peer_addr().unwrap());
        let mut client = PortMapperClient::new(stream);

        assert_eq!(
            client
                .getport(Mapping::new(PORTMAPPER_PROG, PORTMAPPER_VERS, 6, 0))
                .await
                .unwrap(),
            111
        );
    }
}

pub struct StaticPortMapBuilder {
    mappings: Vec<Mapping>,
}

impl StaticPortMapBuilder {
    pub fn new() -> Self {
        let mut mappings = Vec::new();
        mappings.push(Mapping::new(
            PORTMAPPER_PROG,
            PORTMAPPER_VERS,
            PORTMAPPER_PROT_TCP,
            PORTMAPPER_PORT as u32,
        ));
        mappings.push(Mapping::new(
            PORTMAPPER_PROG,
            PORTMAPPER_VERS,
            PORTMAPPER_PROT_UDP,
            PORTMAPPER_PORT as u32,
        ));
        StaticPortMapBuilder { mappings }
    }

    /// Set a mapping
    pub fn set(mut self, mapping: Mapping) -> Self {
        self.mappings.push(mapping);
        self
    }

    pub fn add(&mut self, mapping: Mapping) {
        self.mappings.push(mapping);
    }

    /// Set a mapping
    pub fn build(self) -> Arc<StaticPortMap> {
        Arc::new(StaticPortMap {
            mappings: self.mappings,
        })
    }
}

/// Create a simple static portmapper
///
/// The static portmapper allows null, getport and dump procedures, others will respond with a ProcUnavail error
pub struct StaticPortMap {
    mappings: Vec<Mapping>,
}

impl StaticPortMap {
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
impl RpcService for StaticPortMap {
    async fn call(
        self: Arc<Self>,
        prog: u32,
        vers: u32,
        proc: u32,
        args: &mut Cursor<Vec<u8>>,
        ret: &mut Cursor<Vec<u8>>,
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
                let mut mapping = Mapping::default();
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
