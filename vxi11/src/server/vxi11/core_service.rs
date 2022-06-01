use std::{
    collections::HashMap,
    io::{self, Cursor},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration, cmp::min,
};

use async_listen::ListenExt;
use async_std::{
    net::TcpListener,
    future::timeout, task,
};
use lxi_device::{Device, lock::SharedLockError};

use crate::common::{
    onc_rpc::prelude::*,
    vxi11::{self, xdr},
    xdr::prelude::*,
};

use futures::{
    lock::Mutex,
    select, FutureExt, StreamExt,
};


use super::{intr_client::VxiSrqClient, prelude::*, Link, VxiInner};

macro_rules! get_link {
    ($links:expr, $lid:expr) => {
        $links.lock().await.get_mut($lid)
    };
}

macro_rules! lock_device {
    ($handle:expr, $flags:expr, $timeout:expr, $abort:expr) => {
        if $flags.is_waitlock() {
            select! {
                d = timeout(
                    Duration::from_millis($timeout as u64),
                    $handle.async_lock(),
                ).fuse() => d.map_or(Err(SharedLockError::Timeout), |f| f),
                _ = $abort.next() => Err(SharedLockError::Aborted)
            }
        } else {
            $handle.try_lock()
        }
    };
}

/// Core RPC service
pub struct VxiCoreServer<DEV> {
    pub(super) inner: Arc<Mutex<VxiInner<DEV>>>,
    pub(super) max_recv_size: u32,
    pub(super) async_port: u16,
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
                peer,
                inner: self.inner.clone(),
                max_recv_size: self.max_recv_size,
                async_port: self.async_port,
                links: Mutex::new(HashMap::new()),
                srq: Mutex::new(None),
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

pub struct VxiCoreSession<DEV> {
    peer: SocketAddr,
    inner: Arc<Mutex<VxiInner<DEV>>>,
    max_recv_size: u32,
    async_port: u16,

    // Links created by this session
    // Will be dropped when client disconnects
    links: Mutex<HashMap<u32, Link<DEV>>>,

    srq: Mutex<Option<VxiSrqClient>>,
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
            vxi11::CREATE_LINK => {
                let mut parms = xdr::CreateLinkParms::default();
                parms.read_xdr(args)?;

                let mut resp = xdr::CreateLinkResp {
                    error: xdr::DeviceErrorCode::NoError,
                    lid: 0.into(),
                    abort_port: self.async_port,
                    max_recv_size: self.max_recv_size,
                };

                if parms.device.starts_with("inst") {
                    let (lid, mut link) = {
                        let mut inner = self.inner.lock().await;
                        inner.new_link()
                    };
                    resp.lid = lid.into();

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
                                log::debug!(peer=format!("{}", self.peer), link=lid; "Exclusive lock acquired")
                            }
                            Err(err) => resp.error = err.into(),
                        }
                    }
                    log::debug!(peer=format!("{}", self.peer), link=lid; "New link: {}, client_id={}", parms.device, parms.client_id);
                    self.links.lock().await.insert(lid, link);
                } else {
                    log::debug!(peer=format!("{}", self.peer); "Invalid device address: {}", parms.device);
                    resp.error = xdr::DeviceErrorCode::InvalidAddress;
                }

                resp.write_xdr(ret)?;
                Ok(())
            }
            vxi11::DEVICE_WRITE => {
                // Read parameters
                let mut parms = xdr::DeviceWriteParms::default();
                parms.read_xdr(args)?;

                let mut resp = xdr::DeviceWriteResp::default();

                log::debug!(peer=format!("{}", self.peer), link=parms.lid.0,
                    lock_timeout=parms.lock_timeout,
                    io_timeout=parms.io_timeout,
                    flags=format!("{}", parms.flags); 
                    "Write {:?}", parms.data.0);

                resp.error = match get_link!(self.links, &parms.lid.0) {
                    Some(link) => {
                        // Lock device
                        let dev =
                            lock_device!(link.handle, parms.flags, parms.lock_timeout, link.abort);

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
                                xdr::DeviceErrorCode::NoError
                            }
                            Err(err) => {
                                resp.size = 0;
                                err.into()
                            }
                        }
                    }
                    None => xdr::DeviceErrorCode::InvalidLinkIdentifier,
                };

                // Write response
                resp.write_xdr(ret)?;
                Ok(())
            }
            vxi11::DEVICE_READ => {
                // Read parameters
                let mut parms = xdr::DeviceReadParms::default();
                parms.read_xdr(args)?;

                let mut resp = xdr::DeviceReadResp::default();

                log::debug!(peer=format!("{}", self.peer), link=parms.lid.0,
                    lock_timeout=parms.lock_timeout,
                    io_timeout=parms.io_timeout,
                    flags=format!("{}", parms.flags); 
                    "Read request={:?}, termchar={}", parms.request_size, parms.term_char);

                if let Some(link) = get_link!(self.links, &parms.lid.0) {
                    // Lock device
                    let dev =
                        lock_device!(link.handle, parms.flags, parms.lock_timeout, link.abort);

                    // Execute if END is set
                    resp.error = match dev {
                        Ok(_) => {
                            let to_take = if parms.flags.is_termcharset() {
                                let pos = link.out_buf.iter().position(|c| c.eq(&parms.term_char));

                                // Take whatever is first terminator, or end
                                let to_take = pos.map_or(parms.request_size as usize, |x| {
                                    min(link.out_buf.len(), x + 1)
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
                        Err(err) => err.into(),
                    }
                } else {
                    resp.error = xdr::DeviceErrorCode::InvalidLinkIdentifier;
                };
                log::trace!(link=parms.lid.0; "Read {:?}, size={}, reason={}", resp.error, resp.data.len(), resp.reason);

                // Write response
                resp.write_xdr(ret)?;
                Ok(())
            }
            vxi11::DEVICE_READSTB => {
                // Read parameters
                let mut parms = xdr::DeviceGenericParms::default();
                parms.read_xdr(args)?;

                log::debug!(peer=format!("{}", self.peer), link=parms.lid.0,
                    lock_timeout=parms.lock_timeout,
                    io_timeout=parms.io_timeout,
                    flags=format!("{}", parms.flags); 
                    "Read stb");

                let mut resp = xdr::DeviceReadStbResp::default();

                resp.error = match get_link!(self.links, &parms.lid.0) {
                    Some(link) => {
                        let dev =
                            lock_device!(link.handle, parms.flags, parms.lock_timeout, link.abort);

                        match dev {
                            Ok(mut d) => match d.get_status() {
                                Ok(stb) => {
                                    resp.stb = stb;

                                    // Replace MAV bit
                                    resp.stb &= 0xef;
                                    if !link.out_buf.is_empty() {
                                        resp.stb &= 0x10;
                                    }
                                    xdr::DeviceErrorCode::NoError
                                }
                                Err(err) => err.into(),
                            },
                            Err(err) => err.into(),
                        }
                    }
                    None => xdr::DeviceErrorCode::InvalidLinkIdentifier,
                };

                // Write response
                resp.write_xdr(ret)?;
                Ok(())
            }
            vxi11::DEVICE_TRIGGER => {
                // Read parameters
                let mut parms = xdr::DeviceGenericParms::default();
                parms.read_xdr(args)?;

                log::debug!(peer=format!("{}", self.peer), link=parms.lid.0,
                    lock_timeout=parms.lock_timeout,
                    io_timeout=parms.io_timeout,
                    flags=format!("{}", parms.flags); 
                    "Trigger");

                let mut resp = xdr::DeviceError::default();

                resp.error = match get_link!(self.links, &parms.lid.0) {
                    Some(link) => {
                        let dev =
                            lock_device!(link.handle, parms.flags, parms.lock_timeout, link.abort);

                        match dev {
                            Ok(mut d) => d.trigger().into(),
                            Err(err) => err.into(),
                        }
                    }
                    None => xdr::DeviceErrorCode::InvalidLinkIdentifier,
                };

                // Write response
                resp.write_xdr(ret)?;
                Ok(())
            }
            vxi11::DEVICE_CLEAR => {
                // Read parameters
                let mut parms = xdr::DeviceGenericParms::default();
                parms.read_xdr(args)?;

                log::debug!(peer=format!("{}", self.peer), link=parms.lid.0,
                    lock_timeout=parms.lock_timeout,
                    io_timeout=parms.io_timeout,
                    flags=format!("{}", parms.flags); 
                    "Clear");

                let mut resp = xdr::DeviceError::default();

                resp.error = match get_link!(self.links, &parms.lid.0) {
                    Some(link) => {
                        link.clear();

                        let dev =
                            lock_device!(link.handle, parms.flags, parms.lock_timeout, link.abort);

                        match dev {
                            Ok(mut d) => d.clear().into(),
                            Err(err) => err.into(),
                        }
                    }
                    None => xdr::DeviceErrorCode::InvalidLinkIdentifier,
                };

                // Write response
                resp.write_xdr(ret)?;
                Ok(())
            }
            vxi11::DEVICE_LOCAL | vxi11::DEVICE_REMOTE => {
                // Read parameters
                let mut parms = xdr::DeviceGenericParms::default();
                parms.read_xdr(args)?;

                log::debug!(peer=format!("{}", self.peer), link=parms.lid.0,
                    lock_timeout=parms.lock_timeout,
                    io_timeout=parms.io_timeout,
                    flags=format!("{}", parms.flags); 
                    "Local {}", proc == vxi11::DEVICE_REMOTE);

                let mut resp = xdr::DeviceError::default();

                resp.error = match get_link!(self.links, &parms.lid.0) {
                    Some(link) => {
                        let dev =
                            lock_device!(link.handle, parms.flags, parms.lock_timeout, link.abort);

                        match dev {
                            Ok(mut d) => d.set_remote(proc == vxi11::DEVICE_REMOTE).into(),
                            Err(err) => err.into(),
                        }
                    }
                    None => xdr::DeviceErrorCode::InvalidLinkIdentifier,
                };

                // Write response
                resp.write_xdr(ret)?;
                Ok(())
            }
            vxi11::DEVICE_LOCK => {
                // Read parameters
                let mut parms = xdr::DeviceLockParms::default();
                parms.read_xdr(args)?;

                log::debug!(peer=format!("{}", self.peer), link=parms.lid.0,
                    lock_timeout=parms.lock_timeout,
                    flags=format!("{}", parms.flags); 
                    "Lock");

                let mut resp = xdr::DeviceError::default();

                resp.error = match get_link!(self.links, &parms.lid.0) {
                    Some(link) if parms.flags.is_waitlock() => select! {
                        d = timeout(
                            Duration::from_millis(parms.lock_timeout as u64),
                            link.handle.async_acquire_exclusive(),
                        ).fuse() => d.map_or(Err(SharedLockError::Timeout), |f| f),
                        _ = link.abort.next() => Err(SharedLockError::Aborted)
                    }
                    .into(),
                    Some(link) => link.handle.try_acquire_exclusive().into(),
                    None => xdr::DeviceErrorCode::InvalidLinkIdentifier,
                };

                log::trace!(link=parms.lid.0; "Lock {:?}", resp.error);

                // Write response
                resp.write_xdr(ret)?;
                Ok(())
            }
            vxi11::DEVICE_UNLOCK => {
                // Read parameters
                let mut parms = xdr::DeviceLink::default();
                parms.read_xdr(args)?;

                log::debug!(peer=format!("{}", self.peer), link=parms.0; "Unlock");

                let mut resp = xdr::DeviceError::default();

                resp.error = match get_link!(self.links, &parms.0) {
                    Some(link) => match link.handle.try_release() {
                        Ok(_) => xdr::DeviceErrorCode::NoError,
                        Err(err) => err.into(),
                    },
                    None => xdr::DeviceErrorCode::InvalidLinkIdentifier,
                };

                // Write response
                resp.write_xdr(ret)?;
                Ok(())
            }
            vxi11::DEVICE_ENABLE_SRQ => {
                // Read parameters
                let mut parms = xdr::DeviceEnableSrqParms::default();
                parms.read_xdr(args)?;

                if parms.enable {
                    log::debug!(peer=format!("{}", self.peer), link=parms.lid.0; "Enable srq, handle={:?}", parms.handle);
                } else {
                    log::debug!(peer=format!("{}", self.peer), link=parms.lid.0; "Disable srq");
                }

                let mut resp = xdr::DeviceError::default();

                resp.error = match get_link!(self.links, &parms.lid.0) {
                    Some(link) => {
                        link.srq_enable = parms.enable;
                        // Replace or remove userdata from SRQ handler
                        if parms.handle.is_empty() || !parms.enable {
                            link.srq_handle.take();
                        } else {
                            link.srq_handle.replace(parms.handle.0);
                        }
                        xdr::DeviceErrorCode::NoError
                    }
                    None => xdr::DeviceErrorCode::InvalidLinkIdentifier,
                };

                resp.write_xdr(ret)?;
                Ok(())
            }
            vxi11::DEVICE_DOCMD => {
                // Read parameters
                let mut parms = xdr::DeviceDocmdParms::default();
                parms.read_xdr(args)?;

                let mut resp = xdr::DeviceDocmdResp::default();

                log::debug!(peer=format!("{}", self.peer), link=parms.lid.0; "Docmd {}, data={:?}", parms.cmd, parms.data_in);

                resp.error = xdr::DeviceErrorCode::OperationNotSupported;

                // Write response
                resp.write_xdr(ret)?;
                Ok(())
            }
            vxi11::DESTROY_LINK => {
                // Read parameters
                let mut parms = xdr::DeviceLink::default();
                parms.read_xdr(args)?;

                log::debug!(peer=format!("{}", self.peer), link=parms.0; "Destroy link");

                let mut resp = xdr::DeviceError::default();

                resp.error = match get_link!(self.links, &parms.0) {
                    Some(link) => {
                        let mut inner = self.inner.lock().await;

                        link.handle.force_release();
                        inner.remove_link(parms.0);
                        xdr::DeviceErrorCode::NoError
                    }
                    None => xdr::DeviceErrorCode::InvalidLinkIdentifier,
                };

                resp.write_xdr(ret)?;
                Ok(())
            }
            vxi11::CREATE_INTR_CHAN => {
                // Read parameters
                let mut parms = xdr::DeviceRemoteFunc::default();
                parms.read_xdr(args)?;

                log::debug!(peer=format!("{}", self.peer); "Create interrupt channel, {}, {}, {}, {:?}", SocketAddr::new(Ipv4Addr::from(parms.host_addr).into(),
                    parms.host_port),
                parms.prog_num,
                parms.prog_vers,
                parms.prog_family);

                let mut resp = xdr::DeviceError::default();

                let mut srq = self.srq.lock().await;
                if srq.is_some() {
                    resp.error = xdr::DeviceErrorCode::ChannelAlreadyEstablished
                } else {
                    if let Ok(client) = VxiSrqClient::new(
                        parms.host_addr,
                        parms.host_port,
                        parms.prog_num,
                        parms.prog_vers,
                        parms.prog_family == xdr::DeviceAddrFamily::Udp,
                    )
                    .await
                    {
                        srq.replace(client);
                        resp.error = xdr::DeviceErrorCode::NoError;
                    } else {
                        resp.error = xdr::DeviceErrorCode::ChannelNotEstablished;
                    }
                }

                resp.write_xdr(ret)?;
                Ok(())
            }
            vxi11::DESTROY_INTR_CHAN => {
                // Read parameters
                ().read_xdr(args)?;

                log::debug!(peer=format!("{}", self.peer); "Close interrupt channel");

                let mut resp = xdr::DeviceError::default();

                let mut srq = self.srq.lock().await;
                if srq.take().is_some() {
                    resp.error = xdr::DeviceErrorCode::NoError
                } else {
                    resp.error = xdr::DeviceErrorCode::ChannelNotEstablished
                }

                resp.write_xdr(ret)?;
                Ok(())
            }
            _ => Err(RpcError::ProcUnavail),
        }
    }
}
