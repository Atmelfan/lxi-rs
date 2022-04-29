use std::{
    collections::HashMap,
    io::{Cursor, Error, Read, Write},
    sync::Arc,
};

use async_std::sync::RwLock;
use async_trait::async_trait;
use futures::{AsyncRead, AsyncWrite};

use crate::common::{
    onc_rpc::{RpcClient, RpcError, RpcService},
    xdr::{
        basic::{XdrDecode, XdrEncode},
        onc_rpc::xdr::MissmatchInfo,
        portmapper::{
            PMAPPROC_CALLIT, PMAPPROC_DUMP, PMAPPROC_GETPORT, PMAPPROC_NULL, PMAPPROC_SET,
            PMAPPROC_UNSET, PORTMAPPER_PROG, PORTMAPPER_VERS,
        },
    },
};

pub(crate) use crate::common::xdr::portmapper::{
    xdr::Mapping, PORTMAPPER_PROT_TCP, PORTMAPPER_PROT_UDP,
};

pub(crate) struct PortMapperClient<IO>(RpcClient<IO>);

impl<IO> PortMapperClient<IO>
where
    IO: AsyncRead + AsyncWrite + Unpin,
{
    pub(crate) fn new(io: IO) -> Self {
        Self(RpcClient::new(io))
    }

    pub(crate) async fn null(&mut self, mapping: Mapping) -> Result<bool, RpcError> {
        self.0
            .call(PORTMAPPER_PROG, PORTMAPPER_VERS, PMAPPROC_NULL, mapping)
            .await
    }

    pub(crate) async fn set(&mut self, mapping: Mapping) -> Result<bool, RpcError> {
        self.0
            .call(PORTMAPPER_PROG, PORTMAPPER_VERS, PMAPPROC_SET, mapping)
            .await
    }

    pub(crate) async fn unset(&mut self, mapping: Mapping) -> Result<bool, RpcError> {
        self.0
            .call(PORTMAPPER_PROG, PORTMAPPER_VERS, PMAPPROC_UNSET, mapping)
            .await
    }

    pub(crate) async fn getport(&mut self, mapping: Mapping) -> Result<u16, RpcError> {
        self.0
            .call(PORTMAPPER_PROG, PORTMAPPER_VERS, PMAPPROC_GETPORT, mapping)
            .await
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

struct StaticPortMapBuilder {
    mappings: Vec<Mapping>,
}

impl StaticPortMapBuilder {
    pub fn new() -> Self {
        let mappings = Vec::new();
        mappings.push(Mapping::new(PORTMAPPER_PROG, PORTMAPPER_VERS, PORTMAPPER_PROT_TCP, PORTMAPPER_PORT));
        mappings.push(Mapping::new(PORTMAPPER_PROG, PORTMAPPER_VERS, PORTMAPPER_PROT_UDP, PORTMAPPER_PORT));
        StaticPortMapBuilder { mappings }
    }

    /// Set a mapping
    pub fn set(self, mapping: Mapping) -> Self {
        self.mappings.push(mapping);
        self
    }

    /// Set a mapping
    pub fn build(self) -> Arc<StaticPortMap> {
        Arc::new(StaticPortMap { self.mappings })
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
    pub async fn serve(self: Arc<Self>, addr: IpAddr) -> io::Result<()>{
        try_join!(
            self.clone().serve_tcp((Ipv4Addr::UNSPECIFIED, PORTMAPPER_PORT)),
            self.clone().serve_udp((Ipv4Addr::UNSPECIFIED, PORTMAPPER_PORT))
        )
    }

    /// Serve TCP calls
    pub fn serve_tcp(self: Arc<Self>, addrs: impl ToSocketAddrs) -> io::Result<()> {
        let listener = TcpListener::bind(addrs).await?;
        log::info!("Listening on {}", listener.local_addr());
        let mut incoming = listener
            .incoming()
            .log_warnings(|warn| log::warn!("Listening error: {}", warn))
            .handle_errors(Duration::from_millis(100))
            .backpressure(10);

        while let Some((token, stream)) = incoming.next().await {
            let peer = stream.peer_addr()?;
            println!("Accepted from: {}", peer);

            let s = self.clone();
            task::spawn(async move {
                if let Err(err) = s.serve_tcp(stream).await {
                    log::warn!("Error processing client: {}", err)
                }
                drop(token);
            });
        }
        log::info!("Stopped");
        Ok(())
    }

    /// Serve UDP calls
    pub fn serve_udp(self: Arc<Self>, addrs: impl ToSocketAddrs) -> io::Result<()> {
        let socket = UdpSocket::bind(addrs).await?;
        log::info!("Listening on {}", listener.local_addr());
        loop {
            // Read message
            let mut buf = vec![0; 1500];
            let (n, peer) = socket.recv_from(&mut buf).await?;
        
            let reply = self.handle_message(fragment[..n]).await?;
        
            socket.sendto(reply, peer).await?;
        }
        log::info!("Stopped");
        Ok(())
    }
}

#[async_trait]
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
                let port = self.mappings
                    .iter()
                    .find(|m| m.prog == mapping.prog && m.vers == mapping.vers && m.prot == mapping.prot)
                    .map_or(0, |m| m.port as u16);
                port.write_xdr(ret)?;
                Ok(())
            }
            PMAPPROC_DUMP => {
                let mappings = self.mappings.read().await;
                for mapping in mappings.iter() {
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
