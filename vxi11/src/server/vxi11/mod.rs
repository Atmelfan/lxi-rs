use std::{
    collections::HashMap,
    sync::Arc,
};

use async_std::net::ToSocketAddrs;

use futures::{
    channel::mpsc::{channel, Receiver, Sender},
    lock::Mutex,
};
use lxi_device::{
    lock::{LockHandle, SharedLock, SharedLockError, SpinMutex},
    DeviceError as LxiDeviceError,
};

use crate::{
    client::portmapper::PortMapperClient,
    common::{
        onc_rpc::prelude::*,
        portmapper::{xdr::Mapping, PORTMAPPER_PROT_TCP},
        vxi11::xdr,
    },
};

pub(crate) mod intr_client;
pub(crate) mod core_service;
pub(crate) mod abort_service;

pub mod prelude {
    pub use super::{abort_service::VxiAsyncServer, core_service::VxiCoreServer, VxiServerBuilder};
    pub use crate::common::vxi11::{
        DEVICE_ASYNC, DEVICE_ASYNC_VERSION, DEVICE_CORE, DEVICE_CORE_VERSION, DEVICE_INTR,
        DEVICE_INTR_VERSION,
    };
}


use prelude::*;

use super::portmapper::StaticPortMapBuilder;

impl From<LxiDeviceError> for xdr::DeviceErrorCode {
    fn from(de: LxiDeviceError) -> Self {
        match de {
            LxiDeviceError::NotSupported => xdr::DeviceErrorCode::OperationNotSupported,
            LxiDeviceError::IoTimeout => xdr::DeviceErrorCode::IoTimeout,
            LxiDeviceError::IoError => xdr::DeviceErrorCode::IoError,
            _ => xdr::DeviceErrorCode::DeviceNotAccessible,
        }
    }
}

impl From<SharedLockError> for xdr::DeviceErrorCode {
    fn from(de: SharedLockError) -> Self {
        match de {
            SharedLockError::AlreadyLocked | SharedLockError::AlreadyUnlocked => {
                xdr::DeviceErrorCode::NoLockHeldByThisLink
            }
            SharedLockError::Timeout
            | SharedLockError::LockedByShared
            | SharedLockError::LockedByExclusive => xdr::DeviceErrorCode::DeviceLockedByAnotherLink,
            SharedLockError::Aborted => xdr::DeviceErrorCode::Abort,
            SharedLockError::Busy => xdr::DeviceErrorCode::DeviceNotAccessible,
        }
    }
}

impl<T> From<Result<(), T>> for xdr::DeviceErrorCode
where
    T: Into<xdr::DeviceErrorCode>,
{
    fn from(res: Result<(), T>) -> Self {
        match res {
            Ok(_) => xdr::DeviceErrorCode::NoError,
            Err(err) => err.into(),
        }
    }
}

struct Link<DEV> {
    id: u32,
    handle: LockHandle<DEV>,

    abort: Receiver<()>,

    // Srq
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
                in_buf: Vec::new(),
                out_buf: Vec::new(),
                srq_enable: false,
                srq_handle: None,
            },
            sender,
        )
    }

    fn clear(&mut self) {
        self.in_buf.clear();
        self.out_buf.clear();
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
