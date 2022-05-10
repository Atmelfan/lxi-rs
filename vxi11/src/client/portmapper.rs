use futures::{AsyncRead, AsyncWrite};

use crate::common::{
    onc_rpc::prelude::*,
    portmapper::{
        xdr::Mapping, PMAPPROC_GETPORT, PMAPPROC_NULL, PMAPPROC_SET, PMAPPROC_UNSET,
        PORTMAPPER_PROG, PORTMAPPER_VERS,
    },
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
