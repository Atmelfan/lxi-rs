use std::io::Cursor;

use futures::{AsyncRead, AsyncWrite};

use crate::common::{
    onc_rpc::prelude::*,
    portmapper::{
        xdr::{CallResult, Callit, Mapping},
        PMAPPROC_CALLIT, PMAPPROC_GETPORT, PMAPPROC_NULL, PMAPPROC_SET, PMAPPROC_UNSET,
        PORTMAPPER_PROG, PORTMAPPER_VERS,
    },
    xdr::prelude::*,
};

pub mod prelude {
    pub use super::PortMapperClient;
    pub use crate::common::portmapper::{
        xdr::Mapping, PORTMAPPER_PORT, PORTMAPPER_PROG, PORTMAPPER_PROT_TCP, PORTMAPPER_PROT_UDP,
        PORTMAPPER_VERS,
    };
}

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

    pub async fn callit<ARGS, RET>(
        &mut self,
        prog: u32,
        vers: u32,
        proc: u32,
        args: ARGS,
    ) -> Result<RET, RpcError>
    where
        ARGS: XdrEncode,
        RET: XdrDecode + Default,
    {
        let mut args_cursor = Cursor::new(Vec::new());
        args.write_xdr(&mut args_cursor)?;
        let callit_args = Callit {
            prog,
            vers,
            proc,
            args: args_cursor.into_inner(),
        };
        let res: CallResult = self.0.call(PMAPPROC_CALLIT, callit_args).await?;
        let mut ret: RET = Default::default();
        ret.read_xdr(&mut Cursor::new(res.res))?;
        Ok(ret)
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
