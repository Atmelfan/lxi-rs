use std::{
    collections::HashMap,
    io::{self, Cursor},
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
    time::{Duration, Instant},
};

use async_listen::ListenExt;
use async_std::{
    net::{TcpListener, TcpStream, ToSocketAddrs, UdpSocket},
    task,
};
use futures::{lock::Mutex, StreamExt};
use lxi_device::lock::{LockHandle, SharedLock, SpinMutex};

use crate::{
    client::portmapper::PortMapperClient,
    common::{
        onc_rpc::prelude::*,
        portmapper::{xdr::Mapping, PORTMAPPER_PROT_TCP},
        vxi11::{device_intr_srq, xdr},
    },
};

use crate::common::xdr::prelude::*;

pub mod prelude {
    pub use super::{VxiAsyncServer, VxiCoreServer, VxiServerBuilder};
    pub use crate::common::vxi11::{
        DEVICE_ASYNC, DEVICE_ASYNC_VERSION, DEVICE_CORE, DEVICE_CORE_VERSION, DEVICE_INTR,
        DEVICE_INTR_VERSION,
    };
}

use prelude::*;

use super::portmapper::StaticPortMapBuilder;

struct Link<DEV> {
    id: u32,
    handle: LockHandle<DEV>,

    // Service request
    intr: Option<RpcClient>,
    srq_enable: bool,
    srq_handle: Option<Vec<u8>>,
}

impl<DEV> Link<DEV> {
    fn new(id: u32, handle: LockHandle<DEV>) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            id,
            handle,
            intr: None,
            srq_enable: true,
            srq_handle: None,
        }))
    }

    async fn create_interrupt_channel(
        &mut self,
        host_addr: u32,
        host_port: u16,
        prog_num: u32,
        prog_vers: u32,
        udp: bool,
    ) -> io::Result<()> {
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

        let old = self.intr.replace(client);

        // Close old client (if any)
        drop(old);
        Ok(())
    }

    async fn send_interrupt(&mut self) -> Result<(), RpcError> {
        if !self.srq_enable {
            Ok(())
        } else if let Some(client) = &mut self.intr {
            let mut handle = Vec::new();
            if let Some(h) = &self.srq_handle {
                handle.extend(h)
            }
            let parms = xdr::DeviceSrqParms::new(handle);
            client.call_no_reply(device_intr_srq, parms).await
        } else {
            Ok(())
        }
    }
}

struct VxiInner<DEV> {
    link_id: u32,
    links: HashMap<u32, Arc<Mutex<Link<DEV>>>>,
    shared: Arc<SpinMutex<SharedLock>>,
    device: Arc<Mutex<DEV>>,
}

impl<DEV> VxiInner<DEV> {
    fn new(shared: Arc<SpinMutex<SharedLock>>, device: Arc<Mutex<DEV>>) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            link_id: 0,
            links: HashMap::default(),
            shared,
            device,
        }))
    }

    fn next_link_id(&mut self) -> u32 {
        self.link_id += 1;
        while self.links.contains_key(&self.link_id) {
            self.link_id += 1;
        }
        self.link_id
    }

    fn new_link(&mut self) -> (u32, Arc<Mutex<Link<DEV>>>) {
        let id = self.next_link_id();
        let handle = LockHandle::new(self.shared.clone(), self.device.clone());
        let link = Link::new(id, handle);
        (id, link)
    }

    fn add_link(&mut self, lid: u32, link: Arc<Mutex<Link<DEV>>>) {
        self.links.insert(lid, link);
    }

    fn remove_link(&mut self, lid: u32) -> Option<Arc<Mutex<Link<DEV>>>> {
        self.links.remove(&lid)
    }
}

/// Core RPC service
pub struct VxiCoreServer<DEV> {
    inner: Arc<Mutex<VxiInner<DEV>>>,
    max_recv_size: u32,
    core_port: u16,
    async_port: u16,
}

impl<DEV> VxiCoreServer<DEV>
where
    DEV: Send + 'static,
{
    pub async fn bind(self: Arc<Self>, addrs: IpAddr) -> io::Result<()> {
        let listener = TcpListener::bind((addrs, self.async_port)).await?;
        self.serve(listener).await
    }

    pub async fn serve(self: Arc<Self>, listener: TcpListener) -> io::Result<()> {
        log::info!("Core listening on {}", listener.local_addr()?);
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
                if let Err(err) = s.serve_tcp_stream(stream).await {
                    log::debug!("Error processing client: {}", err)
                }
                drop(token);
            });
        }
        log::info!("Stopped");
        Ok(())
    }
}

#[async_trait::async_trait]
impl<DEV> RpcService for VxiCoreServer<DEV>
where
    DEV: Send,
{
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
        if prog != DEVICE_CORE {
            return Err(RpcError::ProgUnavail);
        }

        if vers != DEVICE_CORE_VERSION {
            return Err(RpcError::ProgMissmatch(MissmatchInfo {
                low: DEVICE_CORE_VERSION,
                high: DEVICE_CORE_VERSION,
            }));
        }

        match proc {
            create_link => {
                let mut inner = self.inner.lock().await;

                let mut parms = xdr::CreateLinkParms::default();
                parms.read_xdr(args)?;

                let (lid, link_arc) = inner.new_link();
                let mut link = link_arc.lock().await;

                let mut resp = xdr::CreateLinkResp {
                    error: xdr::DeviceErrorCode::NoError,
                    lid: lid.into(),
                    abort_port: self.async_port,
                    max_recv_size: self.max_recv_size,
                };

                if parms.device.eq_ignore_ascii_case("inst0") {
                    // Add link
                    inner.add_link(lid, link_arc.clone());
                    drop(inner);

                    // Try to lock
                    // TODO: Await a lock
                    let t1 = Instant::now();
                    while parms.lock_device
                        && Instant::now() - t1 < Duration::from_millis(parms.lock_timeout.into())
                    {
                        if link.handle.try_acquire_exclusive().is_ok() {
                            resp.error = xdr::DeviceErrorCode::NoError;
                            break;
                        } else {
                            resp.error = xdr::DeviceErrorCode::DeviceLockedByAnotherLink;
                        }
                    }
                } else {
                    resp.error = xdr::DeviceErrorCode::InvalidAddress;
                }

                resp.write_xdr(ret)?;
                Ok(())
            }
            device_write => todo!("device_write"),
            device_read => todo!("device_read"),
            device_readstb => todo!("device_readstb"),
            device_trigger => todo!("device_trigger"),
            device_clear => todo!("device_clear"),
            device_remote => todo!("device_remote"),
            device_local => todo!("device_local"),
            device_lock => todo!("device_lock"),
            device_unlock => todo!("device_unlock"),
            device_enable_srq => todo!("device_enable_srq"),
            device_docmd => todo!("device_docmd"),
            destroy_link => {
                let mut inner = self.inner.lock().await;

                // Read parameters
                let mut parms = xdr::DeviceLink::default();
                parms.read_xdr(args)?;

                let mut resp = xdr::DeviceError::default();

                if let Some(link) = inner.links.get(&parms.0) {
                    let mut link = link.lock().await;
                    link.handle.force_release();
                } else {
                    resp.error = xdr::DeviceErrorCode::InvalidLinkIdentifier;
                }
                inner.remove_link(parms.0);

                resp.write_xdr(ret)?;
                Ok(())
            }
            create_intr_chan => todo!("create_intr_chan"),
            destroy_intr_chan => todo!("destroy_intr_chan"),
            _ => Err(RpcError::ProcUnavail),
        }
    }
}

/// Async/abort RPC service
pub struct VxiAsyncServer<DEV> {
    inner: Arc<Mutex<VxiInner<DEV>>>,
    async_port: u16,
}

impl<DEV> VxiAsyncServer<DEV>
where
    DEV: Send + 'static,
{
    pub async fn bind(self: Arc<Self>, addrs: IpAddr) -> io::Result<()> {
        let listener = TcpListener::bind((addrs, self.async_port)).await?;
        self.serve(listener).await
    }

    pub async fn serve(self: Arc<Self>, listener: TcpListener) -> io::Result<()> {
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
                if let Err(err) = s.serve_tcp_stream(stream).await {
                    log::debug!("Error processing client: {}", err)
                }
                drop(token);
            });
        }
        log::info!("Stopped");
        Ok(())
    }
}

#[async_trait::async_trait]
impl<DEV> RpcService for VxiAsyncServer<DEV>
where
    DEV: Send,
{
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
            device_abort => {
                let mut inner = self.inner.lock().await;

                // Read parameters
                let mut parms = xdr::DeviceLink::default();
                parms.read_xdr(args)?;

                let mut resp = xdr::DeviceError::default();

                // TODO

                resp.write_xdr(ret)?;
                Ok(())
            }
            _ => Err(RpcError::ProcUnavail),
        }
    }
}

pub struct VxiServerBuilder {
    core_port: u16,
    async_port: u16,
}

impl VxiServerBuilder {
    pub fn new() -> Self {
        Self {
            core_port: 4322,
            async_port: 4323,
        }
    }

    /// Set the vxi server core port.
    pub fn core_port(mut self, core_port: u16) -> Self {
        self.core_port = core_port;
        self
    }

    /// Set the vxi server async/abort port.
    pub fn async_port(mut self, async_port: u16) -> Self {
        self.async_port = async_port;
        self
    }

    /// Register VXI server using portmap/rpcbind
    pub async fn register_portmap(self, addrs: impl ToSocketAddrs) -> Result<Self, RpcError> {
        if self.async_port == 0 || self.core_port == 0 {
            log::error!("Dynamic port not supported");
            return Err(RpcError::SystemErr);
        }

        let stream = TcpStream::connect(addrs).await?;
        let mut portmap = PortMapperClient::new(stream);
        // Register core service
        let mut res = portmap
            .set(Mapping::new(
                DEVICE_CORE,
                DEVICE_CORE_VERSION,
                PORTMAPPER_PROT_TCP,
                self.core_port as u32,
            ))
            .await?;
        // Register async service
        res &= portmap
            .set(Mapping::new(
                DEVICE_ASYNC,
                DEVICE_ASYNC_VERSION,
                PORTMAPPER_PROT_TCP,
                self.async_port as u32,
            ))
            .await?;
        if res {
            Ok(self)
        } else {
            Err(RpcError::SystemErr)
        }
    }

    /// Register VXI server using [StaticPortMap]
    pub fn register_static_portmap(
        self,
        portmap: &mut StaticPortMapBuilder,
    ) -> Result<Self, RpcError> {
        // Register core service
        portmap.add(Mapping::new(
            DEVICE_CORE,
            DEVICE_CORE_VERSION,
            PORTMAPPER_PROT_TCP,
            self.core_port as u32,
        ));
        // Register async service
        portmap.add(Mapping::new(
            DEVICE_ASYNC,
            DEVICE_ASYNC_VERSION,
            PORTMAPPER_PROT_TCP,
            self.async_port as u32,
        ));
        Ok(self)
    }

    pub fn build<DEV>(
        self,
        shared: Arc<SpinMutex<SharedLock>>,
        device: Arc<Mutex<DEV>>,
    ) -> (Arc<VxiCoreServer<DEV>>, Arc<VxiAsyncServer<DEV>>) {
        let inner = VxiInner::new(shared, device);
        (
            Arc::new(VxiCoreServer {
                inner: inner.clone(),
                core_port: self.core_port,
                async_port: self.async_port,
                max_recv_size: 128 * 1024,
            }),
            Arc::new(VxiAsyncServer {
                inner: inner.clone(),
                async_port: self.async_port,
            }),
        )
    }
}
