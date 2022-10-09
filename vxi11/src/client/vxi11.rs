use std::{io::Cursor, sync::Arc, time::Duration, collections::HashMap};

use async_listen::ListenExt;
use async_std::{
    io,
    net::{TcpListener, TcpStream, ToSocketAddrs, UdpSocket},
    stream::StreamExt,
    task, channel::Sender,
};

use crate::common::{
    onc_rpc::prelude::*,
    vxi11::{
        xdr::{
            CreateLinkParms, CreateLinkResp, DeviceDocmdParms, DeviceDocmdResp,
            DeviceEnableSrqParms, DeviceError, DeviceGenericParms, DeviceLink, DeviceLockParms,
            DeviceReadParms, DeviceReadResp, DeviceReadStbResp, DeviceRemoteFunc, DeviceWriteParms,
            DeviceWriteResp,
        },
        CREATE_INTR_CHAN, CREATE_LINK, DESTROY_INTR_CHAN, DESTROY_LINK, DEVICE_ABORT, DEVICE_ASYNC,
        DEVICE_ASYNC_VERSION, DEVICE_CLEAR, DEVICE_CORE, DEVICE_CORE_VERSION, DEVICE_DOCMD,
        DEVICE_ENABLE_SRQ, DEVICE_INTR_SRQ, DEVICE_LOCAL, DEVICE_LOCK, DEVICE_READ, DEVICE_READSTB,
        DEVICE_REMOTE, DEVICE_TRIGGER, DEVICE_UNLOCK, DEVICE_WRITE,
    },
    xdr::prelude::*,
};

pub struct Vxi11CoreClient(StreamRpcClient<TcpStream>);

impl Vxi11CoreClient {
    pub async fn connect(addrs: impl ToSocketAddrs) -> io::Result<Self> {
        let io = TcpStream::connect(addrs).await?;
        Ok(Self(StreamRpcClient::new(
            io,
            DEVICE_CORE,
            DEVICE_CORE_VERSION,
        )))
    }

    pub async fn create_link(
        &mut self,
        parms: CreateLinkParms,
    ) -> Result<CreateLinkResp, RpcError> {
        self.0.call(CREATE_LINK, parms).await
    }

    pub async fn device_write(
        &mut self,
        parms: DeviceWriteParms,
    ) -> Result<DeviceWriteResp, RpcError> {
        self.0.call(DEVICE_WRITE, parms).await
    }

    pub async fn device_read(
        &mut self,
        parms: DeviceReadParms,
    ) -> Result<DeviceReadResp, RpcError> {
        self.0.call(DEVICE_READ, parms).await
    }

    pub async fn device_readstb(
        &mut self,
        parms: DeviceGenericParms,
    ) -> Result<DeviceReadStbResp, RpcError> {
        self.0.call(DEVICE_READSTB, parms).await
    }

    pub async fn device_trigger(
        &mut self,
        parms: DeviceGenericParms,
    ) -> Result<DeviceError, RpcError> {
        self.0.call(DEVICE_TRIGGER, parms).await
    }

    pub async fn device_clear(
        &mut self,
        parms: DeviceGenericParms,
    ) -> Result<DeviceError, RpcError> {
        self.0.call(DEVICE_CLEAR, parms).await
    }

    pub async fn device_remote(
        &mut self,
        parms: DeviceGenericParms,
    ) -> Result<DeviceError, RpcError> {
        self.0.call(DEVICE_REMOTE, parms).await
    }

    pub async fn device_local(
        &mut self,
        parms: DeviceGenericParms,
    ) -> Result<DeviceError, RpcError> {
        self.0.call(DEVICE_LOCAL, parms).await
    }

    pub async fn device_lock(&mut self, parms: DeviceLockParms) -> Result<DeviceError, RpcError> {
        self.0.call(DEVICE_LOCK, parms).await
    }

    pub async fn device_unlock(&mut self, parms: DeviceLink) -> Result<DeviceError, RpcError> {
        self.0.call(DEVICE_UNLOCK, parms).await
    }

    pub async fn device_enable_srq(
        &mut self,
        parms: DeviceEnableSrqParms,
    ) -> Result<DeviceError, RpcError> {
        self.0.call(DEVICE_ENABLE_SRQ, parms).await
    }

    pub async fn device_docmd(
        &mut self,
        parms: DeviceDocmdParms,
    ) -> Result<DeviceDocmdResp, RpcError> {
        self.0.call(DEVICE_DOCMD, parms).await
    }

    pub async fn destroy_link(&mut self, parms: DeviceLink) -> Result<DeviceError, RpcError> {
        self.0.call(DESTROY_LINK, parms).await
    }

    pub async fn create_intr_chan(
        &mut self,
        parms: DeviceRemoteFunc,
    ) -> Result<DeviceError, RpcError> {
        self.0.call(CREATE_INTR_CHAN, parms).await
    }

    pub async fn destroy_intr_chan(&mut self) -> Result<DeviceError, RpcError> {
        self.0.call(DESTROY_INTR_CHAN, ()).await
    }
}

pub struct Vxi11AsyncClient(StreamRpcClient<TcpStream>);

impl Vxi11AsyncClient {
    pub async fn connect(addrs: impl ToSocketAddrs) -> io::Result<Self> {
        let io = TcpStream::connect(addrs).await?;
        Ok(Self(StreamRpcClient::new(
            io,
            DEVICE_ASYNC,
            DEVICE_ASYNC_VERSION,
        )))
    }

    pub async fn device_abort(&mut self, parms: DeviceLink) -> Result<DeviceError, RpcError> {
        self.0.call(DEVICE_ABORT, parms).await
    }
}

/// Async/abort RPC service
pub struct VxiIntrServer {
    observers: HashMap<u32, Sender<()>>
}

impl VxiIntrServer {
    pub async fn bind(self: Arc<Self>, addrs: impl ToSocketAddrs) -> io::Result<()> {
        let listener = TcpListener::bind(addrs).await?;
        self.serve_tcp(listener).await
    }

    pub async fn serve_udp(self: Arc<Self>, socket: UdpSocket) -> io::Result<()> {
        log::info!("Listening on UDP {:?}", socket.local_addr()?);
        self.serve_udp_socket_noreply(socket).await
    }

    pub async fn serve_tcp(self: Arc<Self>, listener: TcpListener) -> io::Result<()> {
        log::info!("Async listening on {}", listener.local_addr()?);
        let mut incoming = listener
            .incoming()
            .log_warnings(|warn| log::warn!("Listening error: {}", warn))
            .handle_errors(Duration::from_millis(100))
            .backpressure(10);

        while let Some((token, stream)) = incoming.next().await {
            let peer = stream.peer_addr()?;
            log::debug!("Accepted from: {}", peer);

            let s = self.clone();
            task::spawn(async move {
                if let Err(err) = s.serve_tcp_stream_noreply(stream).await {
                    log::debug!("Error processing client: {}", err)
                }
                drop(token);
            });
        }
        log::info!("Stopped");
        Ok(())
    }

    pub fn attach_listener(&mut self, device_link: u32, channel: Sender<()>) {
        self.observers.insert(device_link, channel);
    }

    pub fn remove_listener(&mut self, device_link: u32) {
        self.observers.remove(&device_link);
    }

    pub fn notify(&self, device_link: u32) {
        if let Some(channel) = self.observers.get(&device_link) {
            if let Err(_) = channel.try_send(()) {
                log::debug!("Interrupt from device link {} ignored, channel full/closed", device_link)
            }
        }
    }
}

#[async_trait::async_trait]
impl RpcService for VxiIntrServer {
    async fn call(
        self: Arc<Self>,
        prog: u32,
        vers: u32,
        proc: u32,
        args: &mut Cursor<Vec<u8>>,
        ret: &mut Cursor<Vec<u8>>,
    ) -> Result<(), RpcError>
    where
        Self: Sync,
    {
        if prog != DEVICE_ASYNC {
            return Err(RpcError::ProgUnavail);
        }

        if vers != DEVICE_ASYNC_VERSION {
            return Err(RpcError::ProgMissmatch(MissmatchInfo {
                low: DEVICE_ASYNC_VERSION,
                high: DEVICE_ASYNC_VERSION,
            }));
        }

        match proc {
            0 => Ok(()),
            DEVICE_INTR_SRQ => {
                // Read parameters
                let mut parms = DeviceLink::default();
                parms.read_xdr(args)?;

                // Send a message to listener if any
                self.notify(parms.0);

                // Not actually sent
                ().write_xdr(ret)?;
                Ok(())
            }
            _ => Err(RpcError::ProcUnavail),
        }
    }
}
