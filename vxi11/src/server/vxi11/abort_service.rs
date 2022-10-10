use std::{
    io::{self, Cursor},
    net::IpAddr,
    sync::Arc,
    time::Duration,
};

use async_listen::ListenExt;
use async_std::{net::TcpListener, task};

use crate::common::{
    onc_rpc::prelude::*,
    vxi11::{self, xdr},
    xdr::prelude::*,
};

use futures::{lock::Mutex, StreamExt};

use super::{prelude::*, VxiInner};

/// Async/abort RPC service
pub struct VxiAsyncServer<DEV> {
    pub(super) inner: Arc<Mutex<VxiInner<DEV>>>,
    pub(super) async_port: u16,
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
            vxi11::DEVICE_ABORT => {
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
                        let _ = abort.try_send(());
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
