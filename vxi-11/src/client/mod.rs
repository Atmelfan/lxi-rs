use crate::common::xdr::vxi11::*;

enum VxiClientError {
    Rpc(RpcError),
    Device(DeviceErrorCode)
}

struct Vxi11Client<'call> {
    lid: DeviceLink,
    core_client: RpcClient,
    async_client: RpcClient,
    max_recv_size: u32,
    srq_callback: Option<Box<Fn(Vec<u8>) -> Vec<u8> + 'call>>,
}

impl<'call> Vxi11Client<'call> {
    pub async fn bind(addr: IpAddr, client_id: i32, lock_device: bool, lock_timeout: u32, device: String) -> Result<Self, VxiError> {
        // Setup core RPC client
        log::info!("Connecting to VXI11 @ {:?}", addr);
        let core_port = portmapper::getport(addr, DEVICE_CORE, DEVICE_CORE_VERSION, IPPROTO_TCP)?;
        log::debug!("Core channel @ port {}", core_port);
        let core_client = RpcClient::new((addr, core_port))?;
        
        // Setup link
        let link_parms = CreateLinkParms::new(client_id, lock_device, lock_timeout, device);
        let link_resp: CreateLinkResp = core_client.call(CREATE_LINK, link_parms)?;
        if link_resp.error != DeviceErrorCode::NoError {
            log::error!("Create link returned error: {:?}", link_resp.error);
            return Err(link_resp.error.into());
        }

        // Setup async RPC client
        let async_port = link_resp.abort_port;
        log::debug!("Async channel @ port {}", async_port);
        let async_client = RpcClient::new((addr, async_port))?;

        Self {
            lid: link_resp.lid,
            core_client,
            async_client,
            max_recv_size: link_resp.max_recv_size,
            srq_callback: None
        }
    }


}

#[async_trait]
impl<'call> RpcClient for Vxi11Client<'call> { }

#[async_trait]
impl<'call> RpcService for Vxi11Client<'call> {
    async fn call(
        &mut self,
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
            if let Some(srq_fn) = self.srq_fn {
                srq_fn(parms.handle);
            }
            // The device_intr_srq call is one-way, do not send a reply. 
            Err(RpcError::DoNotReply)
        } else {
            Err(RpcError::ProcUnavail)
        }
    }
}