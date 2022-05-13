#![allow(non_upper_case_globals)]

use std::{
    cmp::min,
    collections::HashMap,
    io::{self, Cursor},
    net::{IpAddr, Ipv4Addr},
    sync::{Arc, Weak},
    time::{Duration, Instant},
};

use async_listen::ListenExt;
use async_std::{
    future::timeout,
    net::{TcpListener, TcpStream, ToSocketAddrs, UdpSocket},
    task,
};
use futures::{
    channel::mpsc::{channel, Receiver, Sender},
    lock::Mutex,
    select, FutureExt, StreamExt,
};
use lxi_device::{
    lock::{LockHandle, SharedLock, SharedLockError, SpinMutex},
    Device,
};

use crate::{
    client::portmapper::PortMapperClient,
    common::{
        onc_rpc::prelude::*,
        portmapper::{xdr::Mapping, PORTMAPPER_PROT_TCP},
        vxi11::*,
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

macro_rules! lock_handle {
    ($handle:expr, $flags:expr, $timeout:expr) => {
        if $flags.is_waitlock() {
            match timeout(Duration::from_millis($timeout), $handle.async_lock()).await {
                Ok(Ok(dev)) => Ok(dev),
                Ok(Err(err)) => Err(err),
                Err(_) => Err(SharedLockError::Timeout),
            }
        } else {
            match $handle.try_lock() {
                Ok(dev) => Ok(dev),
                Err(err) => Err(err),
            }
        }
    };
}

struct Link<DEV> {
    id: u32,
    handle: LockHandle<DEV>,

    abort: Receiver<()>,

    // Service request
    intr: Option<RpcClient>,
    srq_enable: bool,
    srq_handle: Option<Vec<u8>>,

    // Buffers
    in_buf: Vec<u8>,
    out_buf: Vec<u8>,
}

impl<DEV> Link<DEV> {
    fn new(id: u32, handle: LockHandle<DEV>) -> (Self, Sender<()>) {
        let (sender, receiver) = channel(1);
        (
            Self {
                id,
                handle,
                abort: receiver,
                intr: None,
                srq_enable: true,
                srq_handle: None,
                in_buf: Vec::new(),
                out_buf: Vec::new(),
            },
            sender,
        )
    }

    fn clear(&mut self) {
        self.in_buf.clear();
        self.out_buf.clear();
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

        // Replace and close old client (if any)
        let old = self.intr.replace(client);
        drop(old);

        Ok(())
    }

    async fn send_interrupt(&mut self) -> Result<(), RpcError> {
        if !self.srq_enable {
            Ok(())
        } else if let Some(client) = &mut self.intr {
            let parms =
                xdr::DeviceSrqParms::new(Opaque(self.srq_handle.clone().unwrap_or_default()));
            client.call_no_reply(device_intr_srq, parms).await
        } else {
            Ok(())
        }
    }

    fn close(&mut self) {
        log::trace!("Link {} closed", self.id);
        // Release any held locks
        self.handle.force_release();
    }
}

impl<DEV> Drop for Link<DEV> {
    fn drop(&mut self) {
        self.close()
    }
}

struct VxiInner<DEV> {
    link_id: u32,
    links: HashMap<u32, Sender<()>>,
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

    fn new_link(&mut self) -> (u32, Link<DEV>) {
        let id = self.next_link_id();
        let handle = LockHandle::new(self.shared.clone(), self.device.clone());
        let (link, sender) = Link::new(id, handle);
        self.links.insert(id, sender);
        (id, link)
    }

    fn remove_link(&mut self, lid: u32) {
        self.links.remove(&lid);
    }
}

/// Core RPC service
pub struct VxiCoreServer<DEV> {
    inner: Arc<Mutex<VxiInner<DEV>>>,
    max_recv_size: u32,
    async_port: u16,
}

pub struct VxiCoreSession<DEV> {
    inner: Arc<Mutex<VxiInner<DEV>>>,
    max_recv_size: u32,
    async_port: u16,
    // Links created by this session
    // Will be dropped when client disconnects
    links: Mutex<HashMap<u32, Link<DEV>>>,
}

impl<DEV> VxiCoreServer<DEV>
where
    DEV: Device + Send + 'static,
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
            let s = Arc::new(VxiCoreSession {
                inner: self.inner.clone(),
                max_recv_size: self.max_recv_size,
                async_port: self.async_port,
                links: Mutex::new(HashMap::new()),
            });

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
impl<DEV> RpcService for VxiCoreSession<DEV>
where
    DEV: Device + Send + 'static,
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
            0 => Ok(()),
            create_link => {
                let mut parms = xdr::CreateLinkParms::default();
                parms.read_xdr(args)?;

                let (lid, mut link) = {
                    let mut inner = self.inner.lock().await;
                    inner.new_link()
                };

                let mut resp = xdr::CreateLinkResp {
                    error: xdr::DeviceErrorCode::NoError,
                    lid: lid.into(),
                    abort_port: self.async_port,
                    max_recv_size: self.max_recv_size,
                };

                if parms.device.eq_ignore_ascii_case("inst0") {
                    // Try to lock
                    if parms.lock_device {
                        let res = timeout(
                            Duration::from_millis(parms.lock_timeout as u64),
                            link.handle.async_acquire_exclusive(),
                        )
                        .await
                        .map_or(Err(SharedLockError::Timeout), |f| f);
                        match res {
                            Ok(()) => {
                                log::debug!(link=lid; "Exclusive lock acquired")
                            }
                            Err(SharedLockError::Timeout) => {
                                log::debug!(link=lid; "Timed out while waiting for exclusive lock");
                                resp.error = xdr::DeviceErrorCode::DeviceLockedByAnotherLink
                            }
                            Err(err) => {
                                log::debug!(link=lid; "Failed to acquire exclusive lock: {:?}", err);
                                resp.error = xdr::DeviceErrorCode::DeviceLockedByAnotherLink
                            }
                        }
                    }
                    log::debug!(link=lid; "New link: {}", parms.device);
                    self.links.lock().await.insert(lid, link);
                } else {
                    log::debug!(link=lid; "Invalid device address: {}", parms.device);
                    resp.error = xdr::DeviceErrorCode::InvalidAddress;
                }

                resp.write_xdr(ret)?;
                Ok(())
            }
            device_write => {
                // Read parameters
                let mut parms = xdr::DeviceWriteParms::default();
                parms.read_xdr(args)?;

                let mut resp = xdr::DeviceWriteResp::default();

                log::debug!(link=parms.lid.0,
                    lock_timeout=parms.lock_timeout,
                    io_timeout=parms.io_timeout,
                    flags=format!("{}", parms.flags); 
                    "Write {:?}", parms.data.0);

                if let Some(link) = self.links.lock().await.get_mut(&parms.lid.0) {
                    // Lock device
                    let dev = if parms.flags.is_waitlock() {
                        timeout(
                            Duration::from_millis(parms.lock_timeout as u64),
                            link.handle.async_lock(),
                        )
                        .await
                    } else {
                        Ok(link.handle.try_lock())
                    }
                    .map_or(Err(SharedLockError::Timeout), |f| f);

                    // Execute if END is set
                    match dev {
                        Ok(mut dev) => {
                            link.in_buf
                                .try_reserve(parms.data.len())
                                .map_err(|_| RpcError::SystemErr)?;
                            link.in_buf.extend_from_slice(&parms.data);
                            resp.size = parms.data.0.len() as u32;

                            if parms.flags.is_end() {
                                let v = dev.execute(&link.in_buf);
                                //log::debug!(link=parms.lid.0; "Execute {:?} -> {:?}", link.in_buf, v);
                                link.out_buf.extend(&v);
                                link.in_buf.clear();
                            }
                            resp.error = xdr::DeviceErrorCode::NoError;
                        }
                        Err(_) => {
                            resp.size = 0;
                            resp.error = xdr::DeviceErrorCode::DeviceLockedByAnotherLink;
                        }
                    }
                } else {
                    resp.error = xdr::DeviceErrorCode::InvalidLinkIdentifier
                };

                // Write response
                resp.write_xdr(ret)?;
                Ok(())
            }
            device_read => {
                // Read parameters
                let mut parms = xdr::DeviceReadParms::default();
                parms.read_xdr(args)?;

                let mut resp = xdr::DeviceReadResp::default();

                log::debug!(link=parms.lid.0,
                    lock_timeout=parms.lock_timeout,
                    io_timeout=parms.io_timeout,
                    flags=format!("{}", parms.flags); 
                    "Read {:?}", parms.request_size);

                resp.error = if let Some(link) = self.links.lock().await.get_mut(&parms.lid.0) {
                    // Lock device
                    let dev = if parms.flags.is_waitlock() {
                        timeout(
                            Duration::from_millis(parms.lock_timeout as u64),
                            link.handle.async_lock(),
                        )
                        .await
                    } else {
                        Ok(link.handle.try_lock())
                    }
                    .map_or(Err(SharedLockError::Timeout), |f| f);

                    // Execute if END is set
                    match dev {
                        Ok(_) => {
                            let to_take = if parms.flags.is_termcharset() {
                                let pos = link.out_buf.iter().position(|c| c.eq(&parms.term_char));

                                // Take whatever is first terminator, or end
                                let to_take = pos.map_or(parms.request_size as usize, |x| {
                                    x.min(link.out_buf.len())
                                });
                                // Returning because of term_char
                                if matches!(pos, Some(c) if c == to_take) {
                                    resp.reason |= 0x2;
                                }
                                to_take
                            } else {
                                link.out_buf.len()
                            }
                            .min(parms.request_size as usize);

                            // Returning because of request_size
                            if to_take == parms.request_size as usize {
                                resp.reason |= 0x1;
                            }
                            // Returning because of end
                            if to_take == link.out_buf.len() {
                                resp.reason |= 0x4;
                            }
                            let data = link.out_buf.drain(0..to_take);
                            resp.data = Opaque(data.collect());

                            xdr::DeviceErrorCode::NoError
                        }
                        Err(_) => xdr::DeviceErrorCode::DeviceLockedByAnotherLink,
                    }
                } else {
                    xdr::DeviceErrorCode::InvalidLinkIdentifier
                };
                log::debug!(link=parms.lid.0; "Read {:?}, size={}, reason={}", resp.error, resp.data.len(), resp.reason);

                // Write response
                resp.write_xdr(ret)?;
                Ok(())
            }
            device_readstb => {
                // Read parameters
                let mut parms = xdr::DeviceGenericParms::default();
                parms.read_xdr(args)?;

                log::debug!(link=parms.lid.0,
                    lock_timeout=parms.lock_timeout,
                    io_timeout=parms.io_timeout,
                    flags=format!("{}", parms.flags); 
                    "Read stb");

                let mut resp = xdr::DeviceReadStbResp::default();

                if let Some(link) = self.links.lock().await.get_mut(&parms.lid.0) {
                    // try to lock and read
                    let dev = if parms.flags.is_waitlock() {
                        select! {
                            d = timeout(
                                Duration::from_millis(parms.lock_timeout as u64),
                                link.handle.async_lock(),
                            ).fuse() => d.map_or(Err(SharedLockError::Timeout), |f| f),
                            _ = link.abort.next() => Err(SharedLockError::Aborted)
                        }
                    } else {
                        link.handle.try_lock()
                    };

                    match dev {
                        Ok(mut d) => resp.stb = d.get_status(),
                        Err(SharedLockError::Aborted) => resp.error = xdr::DeviceErrorCode::Abort,
                        Err(_) => resp.error = xdr::DeviceErrorCode::DeviceLockedByAnotherLink,
                    }
                } else {
                    resp.error = xdr::DeviceErrorCode::InvalidLinkIdentifier;
                }

                // Write response
                resp.write_xdr(ret)?;
                Ok(())
            }
            device_trigger => todo!("device_trigger"),
            device_clear => todo!("device_clear"),
            device_remote => todo!("device_remote"),
            device_local => todo!("device_local"),
            device_lock => {
                // Read parameters
                let mut parms = xdr::DeviceLockParms::default();
                parms.read_xdr(args)?;

                log::debug!(link=parms.lid.0,
                    lock_timeout=parms.lock_timeout,
                    flags=format!("{}", parms.flags); 
                    "Lock");

                let mut resp = xdr::DeviceError::default();

                if let Some(link) = self.links.lock().await.get_mut(&parms.lid.0) {
                    // try to lock and read
                    let dev = if parms.flags.is_waitlock() {
                        select! {
                            d = timeout(
                                Duration::from_millis(parms.lock_timeout as u64),
                                link.handle.async_acquire_exclusive(),
                            ).fuse() => d.map_or(Err(SharedLockError::Timeout), |f| f),
                            _ = link.abort.next() => Err(SharedLockError::Aborted)
                        }
                    } else {
                        link.handle.try_acquire_exclusive()
                    };

                    resp.error = match dev {
                        Ok(()) => xdr::DeviceErrorCode::NoError,
                        Err(SharedLockError::Aborted) => xdr::DeviceErrorCode::Abort,
                        Err(_) => xdr::DeviceErrorCode::DeviceLockedByAnotherLink,
                    }
                } else {
                    resp.error = xdr::DeviceErrorCode::InvalidLinkIdentifier;
                }

                log::debug!(link=parms.lid.0; "Lock {:?}", resp.error);

                // Write response
                resp.write_xdr(ret)?;
                Ok(())
            }
            device_unlock => {
                // Read parameters
                let mut parms = xdr::DeviceLink::default();
                parms.read_xdr(args)?;

                log::debug!(link=parms.0; "Unlock");

                let mut resp = xdr::DeviceError::default();

                if let Some(link) = self.links.lock().await.get_mut(&parms.0) {
                    resp.error = match link.handle.try_release() {
                        Ok(_) => xdr::DeviceErrorCode::NoError,
                        Err(_) => xdr::DeviceErrorCode::NoLockHeldByThisLink,
                    }
                } else {
                    resp.error = xdr::DeviceErrorCode::InvalidLinkIdentifier;
                }

                // Write response
                resp.write_xdr(ret)?;
                Ok(())
            }
            device_enable_srq => todo!("device_enable_srq"),
            device_docmd => {
                // Read parameters
                let mut parms = xdr::DeviceDocmdParms::default();
                parms.read_xdr(args)?;

                let mut resp = xdr::DeviceDocmdResp::default();

                log::debug!(link=parms.lid.0; "Docmd {}, data={:?}", parms.cmd, parms.data_in);

                resp.error = xdr::DeviceErrorCode::OperationNotSupported;

                // Write response
                resp.write_xdr(ret)?;
                Ok(())
            }
            destroy_link => {
                let mut inner = self.inner.lock().await;

                // Read parameters
                let mut parms = xdr::DeviceLink::default();
                parms.read_xdr(args)?;

                log::debug!(link=parms.0; "Destroy link");

                let mut resp = xdr::DeviceError::default();

                if let Some(link) = self.links.lock().await.get_mut(&parms.0) {
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
            0 => Ok(()),
            device_abort => {
                // Read parameters
                let mut parms = xdr::DeviceLink::default();
                parms.read_xdr(args)?;

                let mut resp = xdr::DeviceError::default();

                // TODO
                let sender = {
                    let inner = self.inner.lock().await;
                    inner.links.get(&parms.0).cloned()
                };

                resp.error = match sender {
                    Some(mut abort) => {
                        abort.try_send(());
                        xdr::DeviceErrorCode::NoError
                    }
                    None => xdr::DeviceErrorCode::InvalidLinkIdentifier,
                };
                resp.write_xdr(ret)?;
                Ok(())
            }
            _ => Err(RpcError::ProcUnavail),
        }
    }
}

/// Builder used to create a VXI11 server
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

        let mut portmap = PortMapperClient::connect_tcp(addrs).await?;
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
