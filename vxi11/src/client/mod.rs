use std::{
    io::{self, Cursor},
    net::IpAddr,
};

use async_std::net::TcpStream;

use crate::{
    common::{
        onc_rpc::{RpcClient, RpcError, RpcService},
        xdr::{
            onc_rpc::xdr::MissmatchInfo,
            portmapper::PORTMAPPER_PORT,
            vxi11::{
                xdr::{
                    CreateLinkParms, CreateLinkResp, DeviceErrorCode, DeviceLink, DeviceSrqParms,
                },
                *,
            }, basic::{XdrDecode, XdrEncode},
        },
    },
    server::portmapper::{Mapping, PortMapperClient, PORTMAPPER_PROT_TCP},
};

enum VxiClientError {
    Rpc(RpcError),
    Device(DeviceErrorCode),
}

impl From<RpcError> for VxiClientError {
    fn from(rpc: RpcError) -> Self {
        Self::Rpc(rpc)
    }
}

impl From<DeviceErrorCode> for VxiClientError {
    fn from(dev: DeviceErrorCode) -> Self {
        Self::Device(dev)
    }
}

impl From<io::Error> for VxiClientError {
    fn from(io: io::Error) -> Self {
        Self::Rpc(RpcError::from(io))
    }
}

struct Vxi11CoreClient<IO> {
    lid: DeviceLink,
    rpc_client: RpcClient<IO>,
    max_recv_size: u32,
    async_port: u16,
}

impl Vxi11CoreClient<TcpStream> {
    /// Create a new client and connect to the core channel
    pub async fn connect(
        addr: IpAddr,
        client_id: i32,
        lock_device: bool,
        lock_timeout: u32,
        device: String,
    ) -> Result<Self, VxiClientError> {
        // Get port of core channel
        let stream = TcpStream::connect((addr, PORTMAPPER_PORT)).await?;
        let mut portmap = PortMapperClient::new(stream);
        let core_port = portmap
            .getport(Mapping::new(
                DEVICE_CORE,
                DEVICE_CORE_VERSION,
                PORTMAPPER_PROT_TCP,
                0,
            ))
            .await?;
        log::debug!("Core channel @ port {}", core_port);

        let stream = TcpStream::connect((addr, core_port)).await?;
        let mut core_client = RpcClient::new(stream);

        // Setup link
        let link_parms = CreateLinkParms {
            client_id,
            lock_device,
            lock_timeout,
            device,
        };
        let link_resp: CreateLinkResp = core_client
            .call(DEVICE_CORE, DEVICE_CORE_VERSION, create_link, link_parms)
            .await?;
        if link_resp.error == DeviceErrorCode::NoError {
            Ok(Self {
                lid: link_resp.lid,
                rpc_client: core_client,
                max_recv_size: link_resp.max_recv_size,
                async_port: link_resp.abort_port,
            })
        } else {
            log::error!("Create link returned error: {:?}", link_resp.error);
            Err(link_resp.error.into())
        }
    }

    /// Create a new client and connect to the async/abort channel.
    /// Can only be done after the core channel has been initialized
    pub async fn connect_async(
        &self
        addr: IpAddr
    ) -> Result<Vxi11AsyncClient, VxiClientError> {
        let stream = TcpStream::connect((addr, self.async_port)).await?;
        let mut async_client = RpcClient::new(stream);
        Ok(Vxi11AsyncClient {
            lid: self.lid,
            rpc_client: async_client
        })
    }
}


struct Vxi11AsyncClient<IO> {
    lid: DeviceLink,
    rpc_client: RpcClient<IO>,
}


struct Vxi11IntrServer {
    lid: DeviceLink,
}

#[async_trait::async_trait]
impl RpcService for Vxi11IntrServer {
    async fn call(
        &self,
        prog: u32,
        vers: u32,
        proc: u32,
        args: &mut Cursor<Vec<u8>>,
        ret: &mut Cursor<Vec<u8>>,
    ) -> Result<(), RpcError> {
        if prog != DEVICE_INTR {
            return Err(RpcError::ProgUnavail);
        }
        if vers != DEVICE_INTR_VERSION {
            return Err(RpcError::ProgMissmatch(MissmatchInfo {
                low: DEVICE_INTR_VERSION,
                high: DEVICE_INTR_VERSION,
            }));
        }
        if proc == device_intr_srq {
            let mut parms = DeviceSrqParms::default();
            parms.read_xdr(args)?;
            // TODO
            ().write_xdr(ret)?;
            Ok(())
        } else {
            Err(RpcError::ProcUnavail)
        }
    }
}