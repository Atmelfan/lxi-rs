use std::io::{Error, Read, Write, Cursor};

use async_trait::async_trait;

use crate::common::{
    onc_rpc::{RpcError, RpcService},
    xdr::{
        onc_rpc::xdr::MissmatchInfo,
        vxi11::{xdr::DeviceLink, *}, basic::{XdrDecode, XdrEncode},
    },
};

struct Vxi11Service {

}

impl Vxi11Service {
    fn new() -> Self {
        Self {

        }
    }

    async fn call_core(
        &mut self,
        vers: u32,
        proc: u32,
        args: &mut Cursor<Vec<u8>>,
        ret: &mut Cursor<Vec<u8>>,
    ) -> Result<(), RpcError> {
        if vers != DEVICE_CORE_VERSION {
            return Err(RpcError::ProgMissmatch(MissmatchInfo {
                low: DEVICE_CORE_VERSION,
                high: DEVICE_CORE_VERSION,
            }));
        }

        Ok(())
    }
}

#[async_trait]
impl RpcService for Vxi11Service {
    async fn call(
        &mut self,
        prog: u32,
        vers: u32,
        proc: u32,
        args: &mut Cursor<Vec<u8>>,
        ret: &mut Cursor<Vec<u8>>,
    ) -> Result<(), RpcError> {
        match prog {
            DEVICE_CORE => self.call_core(vers, proc, args, ret).await,
            DEVICE_ASYNC => self.call_core(vers, proc, args, ret).await,
            DEVICE_CORE => self.call_core(vers, proc, args, ret).await,
            _ => Err(RpcError::ProgUnavail)
        }
    }
}
