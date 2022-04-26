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
            .call(PORTMAPPER_PROG, PORTMAPPER_VERS, PMAPPROC_SET, mapping)
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
        let mut stream = async_std::net::TcpStream::connect("127.0.0.1:111")
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

struct PortMapper {
    mappings: Arc<RwLock<Vec<Mapping>>>,
}

impl PortMapper {
    pub fn new(mappings: Vec<Mapping>) -> Self {
        Self {
            mappings: Arc::new(RwLock::new(mappings)),
        }
    }

    async fn set(&self, mapping: Mapping) -> bool {
        let mut x = self.mappings.write().await;
        if let Some(_m) = &x
            .iter()
            .find(|m| m.prog == mapping.prog && m.vers == mapping.vers && m.prot == mapping.prot)
        {
            false
        } else {
            x.push(mapping);
            true
        }
    }

    async fn unset(&self, mapping: Mapping) -> bool {
        let mut x = self.mappings.write().await;
        x.retain(|m| m.prog != mapping.prog || m.vers != mapping.vers);
        true
    }

    async fn getport(&self, mapping: Mapping) -> u16 {
        self.mappings
            .read()
            .await
            .iter()
            .find(|m| m.prog == mapping.prog && m.vers == mapping.vers && m.prot == mapping.prot)
            .map_or(0, |m| m.port as u16)
    }
}

#[async_trait]
impl RpcService for PortMapper {
    async fn call(
        &self,
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
            PMAPPROC_SET => {
                let mut mapping = Mapping::default();
                mapping.read_xdr(args)?;
                let res = self.set(mapping).await;
                res.write_xdr(ret)?;
                Ok(())
            }
            PMAPPROC_UNSET => {
                let mut mapping = Mapping::default();
                mapping.read_xdr(args)?;
                let res = self.unset(mapping).await;
                res.write_xdr(ret)?;
                Ok(())
            }
            PMAPPROC_GETPORT => {
                let mut mapping = Mapping::default();
                mapping.read_xdr(args)?;
                let res = self.getport(mapping).await;
                res.write_xdr(ret)?;
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
            PMAPPROC_CALLIT => {
                log::error!("PMAPPROC_CALLIT not implemented");
                Err(RpcError::ProcUnavail)
            }
            _ => Err(RpcError::ProcUnavail),
        }
    }
}
