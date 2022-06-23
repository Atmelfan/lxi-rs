use std::io;

use async_std::net::{TcpStream, ToSocketAddrs, UdpSocket};
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

pub struct PortMapperClient(RpcClient);

impl PortMapperClient {
    pub async fn connect_tcp(addrs: impl ToSocketAddrs) -> io::Result<Self> {
        let io = TcpStream::connect(addrs).await?;
        Ok(Self(RpcClient::Tcp(StreamRpcClient::new(
            io,
            PORTMAPPER_PROG,
            PORTMAPPER_VERS,
        ))))
    }

    pub async fn connect_udp(addrs: impl ToSocketAddrs) -> io::Result<Self> {
        let sock = UdpSocket::bind("127.0.0.1:0").await?;
        sock.connect(addrs).await?;
        Ok(Self(RpcClient::Udp(UdpRpcClient::new(
            PORTMAPPER_PROG,
            PORTMAPPER_VERS,
            sock,
        ))))
    }

    pub async fn register(&mut self, mapping: Mapping) -> Result<(), RpcError> {
        let res = self.unset(mapping).await?;
        if !res {
            return Err(RpcError::Portmap);
        }
        let res = self.set(mapping).await?;
        if !res {
            return Err(RpcError::Portmap);
        }
        Ok(())
    }


    pub async fn null(&mut self) -> Result<(), RpcError> {
        self.0.call(PMAPPROC_NULL, ()).await
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
        let mut args_cursor = io::Cursor::new(Vec::new());
        args.write_xdr(&mut args_cursor)?;
        let callit_args = Callit {
            prog,
            vers,
            proc,
            args: args_cursor.into_inner(),
        };
        let res: CallResult = self.0.call(PMAPPROC_CALLIT, callit_args).await?;
        let mut ret: RET = Default::default();
        ret.read_xdr(&mut io::Cursor::new(res.res))?;
        Ok(ret)
    }
}
