use std::{
    collections::HashMap,
    io::{self, Cursor, Error, Read, Write},
    sync::{atomic::AtomicU32, Arc},
    time::{Duration, Instant},
};

use async_listen::ListenExt;
use async_std::{
    net::{TcpListener, TcpStream, ToSocketAddrs},
    task,
};
use async_trait::async_trait;
use futures::{lock::Mutex, AsyncReadExt, StreamExt};
use lxi_device::{
    lock::{LockHandle, SharedLock, SpinMutex},
    Device,
};

use crate::common::{
    onc_rpc::{RpcError, RpcService},
    xdr::{
        basic::{XdrDecode, XdrEncode},
        onc_rpc::xdr::MissmatchInfo,
        portmapper::xdr::Mapping,
        vxi11::{
            xdr::{CreateLinkParms, CreateLinkResp, DeviceErrorCode, DeviceLink, DeviceError},
            *,
        },
    },
};

use super::portmapper::{PortMapperClient, PORTMAPPER_PROT_TCP, StaticPortMap, StaticPortMapBuilder};

struct Link<DEV> {
    id: u32,
    handle: LockHandle<DEV>,
}

impl<DEV> Link<DEV> {
    fn new(id: u32, handle: LockHandle<DEV>) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self { id, handle }))
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
    async_port: u16,
}

impl<DEV> VxiCoreServer<DEV> {

}

#[async_trait]
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
    ) -> Result<(), RpcError> where Self: Sync {
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



                let mut parms = CreateLinkParms::default();
                parms.read_xdr(args)?;

                let (lid, link_arc) = inner.new_link();
                let mut link = link_arc.lock().await;

                let mut resp = CreateLinkResp {
                    error: DeviceErrorCode::NoError,
                    lid: lid.into(),
                    abort_port: self.async_port,
                    max_recv_size: self.max_recv_size,
                };

                if parms.device.eq_ignore_ascii_case("instr0") {
                    // Add link
                    inner.add_link(lid, link_arc.clone());

                    // Try to lock
                    // TODO: Await a lock
                    let t1 = Instant::now();
                    while parms.lock_device
                        && Instant::now() - t1 < Duration::from_millis(parms.lock_timeout.into())
                    {
                        if link.handle.try_acquire_exclusive().is_ok() {
                            resp.error = DeviceErrorCode::NoError;
                            break;
                        } else {
                            resp.error = DeviceErrorCode::DeviceLockedByAnotherLink;
                        }
                    }
                } else {
                    resp.error = DeviceErrorCode::InvalidAddress;
                }

                resp.write_xdr(ret)?;
                Ok(())
            }
            destroy_link => {
                let mut inner = self.inner.lock().await;

                // Read parameters
                let mut parms = DeviceLink::default();
                parms.read_xdr(args)?;

                let mut resp = DeviceError::default();

                if let Some(link) = inner.links.get(&parms.0) {
                    let mut link = link.lock().await;
                    link.handle.force_release();
                } else {
                    resp.error = DeviceErrorCode::InvalidLinkIdentifier;
                }
                inner.remove_link(parms.0);

                resp.write_xdr(ret)?;
                Ok(())
            }
            _ => Err(RpcError::ProcUnavail),
        }
    }
}

/// Async/abort RPC service
pub struct VxiAsyncServer<DEV> {
    inner: Arc<Mutex<VxiInner<DEV>>>,
}

impl<DEV> VxiAsyncServer<DEV> {
    pub async fn serve(self: Arc<Self>, addr: impl ToSocketAddrs) -> io::Result<()>
    where
        DEV: Device + Send + 'static,
    {
        let listener = TcpListener::bind(addr).await?;
        let mut incoming = listener
            .incoming()
            .log_warnings(|warn| log::warn!("Listening error: {}", warn))
            .handle_errors(Duration::from_millis(100))
            .backpressure(10);

        while let Some((token, stream)) = incoming.next().await {
            let s = self.clone();
            let peer = stream.peer_addr()?;
            log::error!("Accepted from: {}", peer);

            let inner = self.inner.clone();

            task::spawn(async move {
                let (reader, writer) = stream.split();
                //TODO
                drop(token);
            });
        }
        Ok(())
    }
}

#[async_trait]
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
    ) -> Result<(), RpcError> where Self: Sync {
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
            core_port: 0,
            async_port: 0,
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
        let stream = TcpStream::connect(addrs).await?;
        let mut portmap = PortMapperClient::new(stream);
        // Register core service
        portmap
            .set(Mapping::new(
                DEVICE_CORE,
                DEVICE_CORE_VERSION,
                PORTMAPPER_PROT_TCP,
                self.core_port as u32,
            ))
            .await?;
        // Register async service
        portmap
            .set(Mapping::new(
                DEVICE_ASYNC,
                DEVICE_ASYNC_VERSION,
                PORTMAPPER_PROT_TCP,
                self.async_port as u32,
            ))
            .await?;
        Ok(self)
    }

    /// Register VXI server using [StaticPortMap]
    pub fn register_static_portmap(self, portmap: &mut StaticPortMapBuilder) -> Result<Self, RpcError> {
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
    ) -> (VxiCoreServer<DEV>, VxiAsyncServer<DEV>) {
        let inner = VxiInner::new(shared, device);
        (
            VxiCoreServer {
                inner: inner.clone(),
                async_port: self.async_port,
                max_recv_size: 128*1024,
            },
            VxiAsyncServer {
                inner: inner.clone(),
            },
        )
    }
}
