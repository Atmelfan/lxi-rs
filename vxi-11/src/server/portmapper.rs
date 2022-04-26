use std::io::{Error, Read, Write, Cursor};

use async_trait::async_trait;

use crate::common::{
    onc_rpc::{RpcError, RpcService},
    xdr::{
        onc_rpc::xdr::MissmatchInfo,
        portmapper::{xdr::Mapping, *}, basic::{XdrDecode, XdrEncode},
    },
};

struct PortMapper {
    mappings: Vec<Mapping>,
}

impl PortMapper {
    fn new() -> Self {
        Self {
            mappings: Vec::new(),
        }
    }

    fn set(&mut self, mapping: Mapping) -> bool {

    }

    fn unset(&mut self, mapping: Mapping) -> bool {
        
    }

    fn getport(&mut self, mapping: Mapping) -> u16 {
        
    }
}

#[async_trait]
impl RpcService for PortMapper {
    async fn call(
        &mut self,
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
            PMAPPROC_NULL => Self::nullproc(self).await,
            PMAPPROC_SET => {
                let mut mapping = Mapping::default();
                mapping.read_xdr(args)?;
                let res = self.set(mapping);
                res.write_xdr(ret)?;
                Ok(())
            },
            PMAPPROC_UNSET => {
                let mut mapping = Mapping::default();
                mapping.read_xdr(args)?;
                let res = self.unset(mapping);
                res.write_xdr(ret)?;
                Ok(())
            },
            PMAPPROC_GETPORT => Self::nullproc(self).await,
            PMAPPROC_DUMP => Self::nullproc(self).await,
            PMAPPROC_CALLIT => Self::nullproc(self).await,
            _ => Err(RpcError::ProcUnavail)
        }
    }
}
