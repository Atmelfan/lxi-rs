use std::{io, net::Ipv4Addr};

use async_std::net::{TcpStream, UdpSocket};

use crate::common::{
    onc_rpc::prelude::*,
    vxi11::{xdr, DEVICE_INTR_SRQ},
    xdr::prelude::*,
};

pub(crate) struct VxiSrqClient {
    // Service request
    client: RpcClient,
}

impl VxiSrqClient {
    pub(crate) async fn device_intr_srq(&mut self, handle: &[u8]) -> Result<(), RpcError> {
        let args = xdr::DeviceSrqParms::new(Opaque(handle.to_vec()));
        self.client.call_no_reply(DEVICE_INTR_SRQ, args).await
    }
}

impl VxiSrqClient {
    pub(crate) async fn new(
        host_addr: u32,
        host_port: u16,
        prog_num: u32,
        prog_vers: u32,
        udp: bool,
    ) -> io::Result<Self> {
        let client = if udp {
            let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await?;
            socket
                .connect((Ipv4Addr::from(host_addr), host_port))
                .await?;
            RpcClient::Udp(UdpRpcClient::new(prog_num, prog_vers, socket))
        } else {
            let stream = TcpStream::connect((Ipv4Addr::from(host_addr), host_port)).await?;
            RpcClient::Tcp(StreamRpcClient::new(stream, prog_num, prog_vers))
        };
        Ok(Self { client })
    }
}
