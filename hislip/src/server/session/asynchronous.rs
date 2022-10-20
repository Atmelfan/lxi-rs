use std::io;
use std::str::from_utf8;
use std::time::Duration;

use async_std::channel::Sender;
use async_std::future;
use async_std::prelude::StreamExt;
use async_std::sync::Arc;
use byteorder::{ByteOrder, NetworkEndian};
use futures::future::Either;
use futures::lock::Mutex;
use futures::{pin_mut, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, FutureExt, Stream};
use lxi_device::lock::{LockHandle, SharedLockError, SharedLockMode, SpinMutex};
use lxi_device::{Device, DeviceError};

use crate::common::errors::{Error, FatalErrorCode, NonFatalErrorCode};
use crate::common::messages::{prelude::*, send_fatal, send_nonfatal};
use crate::common::{Protocol, PROTOCOL_2_0};

use super::{ServerConfig, SharedSession};

pub(crate) struct AsyncSession<DEV>
where
    DEV: Device,
{
    /// Session ID
    id: u16,

    // Config
    config: ServerConfig,

    /// Shared resources
    shared: Arc<Mutex<SharedSession>>,

    /// Device
    handle: Arc<SpinMutex<LockHandle<DEV>>>,

    clear: Sender<()>,
}

impl<DEV> AsyncSession<DEV>
where
    DEV: Device,
{
    pub(crate) fn new(
        id: u16,
        config: ServerConfig,
        shared: Arc<Mutex<SharedSession>>,
        handle: Arc<SpinMutex<LockHandle<DEV>>>,
        clear: Sender<()>,
    ) -> Self {
        Self {
            id,
            config,
            shared,
            handle,
            clear,
        }
    }

    pub(crate) async fn handle_session<S, SRQ>(
        self,
        stream: S,
        peer: String,
        mut srq: SRQ,
        protocol: Protocol,
    ) -> Result<(), io::Error>
    where
        S: AsyncRead + AsyncWrite + Unpin,
        SRQ: Stream<Item = u8> + Unpin,
    {
        let (mut rd, mut wr) = stream.split();
        let mut srq_bit = false;

        loop {
            let read_msg = Message::read_from(&mut rd, self.config.max_message_size).fuse();
            pin_mut!(read_msg);

            let t = match futures::future::select(read_msg, srq.next()).await {
                // Message was received
                Either::Left((msg, _)) => msg,
                // Status changed
                Either::Right((stb, read_msg)) => {
                    // Send SRQ
                    match stb {
                        Some(val) if !srq_bit => {
                            srq_bit = true;
                            MessageType::AsyncServiceRequest
                                .message_params(val, 0)
                                .write_to(&mut wr)
                                .await?
                        }
                        _ => {
                            send_fatal!(peer=peer.to_string(), session_id=self.id;
                                &mut wr, FatalErrorCode::UnidentifiedError,
                                "Server shutdown",
                            );
                        }
                    }
                    // Finish receiving message
                    read_msg.await
                }
            }?;

            match t {
                Ok(msg) => {
                    match msg {
                        Message {
                            message_type: MessageType::VendorSpecific(code),
                            ..
                        } => {
                            send_nonfatal!(peer=peer.to_string(), session_id=self.id;
                                &mut wr, NonFatalErrorCode::UnrecognizedVendorDefinedMessage,
                                "Unrecognized Vendor Defined Message ({})", code
                            );
                        }
                        Message {
                            message_type: typ @ MessageType::Error | typ @ MessageType::FatalError,
                            control_code,
                            payload,
                            ..
                        } => {
                            if typ == MessageType::FatalError {
                                log::error!(peer=peer.to_string(), session_id=self.id;
                                    "Client fatal error {:?}: {}", FatalErrorCode::from_error_code(control_code),
                                    from_utf8(&payload).unwrap_or("<invalid utf8>")
                                );
                            } else {
                                log::warn!(peer=peer.to_string(), session_id=self.id;
                                    "Client error {:?}: {}", NonFatalErrorCode::from_error_code(control_code),
                                    from_utf8(&payload).unwrap_or("<invalid utf8>")
                                );
                            }
                        }
                        Message {
                            message_type: MessageType::AsyncLock,
                            message_parameter,
                            control_code,
                            payload: lockstr,
                        } => {
                            if control_code == 0 {
                                // Release
                                let message_id = message_parameter;
                                log::debug!(peer=peer.to_string(), session_id=self.id, message_id=message_id; "Release async lock");
                                let mut handle = self.handle.lock();
                                let control = match handle.try_release() {
                                    Ok(SharedLockMode::Exclusive) => {
                                        ReleaseLockControl::SuccessExclusive
                                    }
                                    Ok(SharedLockMode::Shared) => ReleaseLockControl::SuccessShared,
                                    Err(_) => ReleaseLockControl::Error,
                                };
                                MessageType::AsyncLockResponse
                                    .message_params(control as u8, 0)
                                    .no_payload()
                                    .write_to(&mut wr)
                                    .await?;
                            } else {
                                // Lock
                                let timeout = message_parameter;

                                let control = match from_utf8(&lockstr) {
                                    Ok(mut lockstr) => {
                                        // Remove null termination (looking at you NI!)
                                        if lockstr.ends_with('\0') {
                                            log::warn!(peer=peer.to_string(), session_id=self.id; "Ignoring null-termination on lockstr");
                                            lockstr = lockstr.trim_end_matches('\0');
                                        }

                                        log::debug!(peer=peer.to_string(), session_id=self.id, timeout=timeout; "Async lock: {:?}", lockstr);
                                        // Try to acquire lock
                                        let mut handle = self.handle.lock();
                                        let res = if timeout == 0 {
                                            // Try to lock immediately
                                            handle.try_acquire(lockstr.as_bytes())
                                        } else {
                                            // Try to acquire lock before timeout
                                            // TODO: Cannot be cancelled by AsyncClearDevice
                                            future::timeout(
                                                Duration::from_millis(timeout as u64),
                                                handle.async_acquire(lockstr.as_bytes()),
                                            )
                                            .await
                                            .map_err(|_| SharedLockError::Timeout)
                                            .and_then(|res| res)
                                        };

                                        //log::debug!(session_id = self.id; "Async lock: {:?}", res);
                                        res.map_or_else(
                                            |err| err.into(),
                                            |_| RequestLockControl::Success,
                                        )
                                    }
                                    Err(_s) => {
                                        log::error!(peer=peer.to_string(), session_id=self.id; "Async lock string is not valid");
                                        RequestLockControl::Error
                                    }
                                };

                                MessageType::AsyncLockResponse
                                    .message_params(control as u8, 0)
                                    .no_payload()
                                    .write_to(&mut wr)
                                    .await?;
                            }
                        }
                        Message {
                            message_type: MessageType::AsyncRemoteLocalControl,
                            control_code: request,
                            message_parameter: message_id,
                            ..
                        } => {
                            log::debug!(peer=peer.to_string(), session_id=self.id, message_id=message_id; "Remote/local request = {}", request);
                            let mut shared = self.shared.lock().await;
                            let handle = self.handle.lock();
                            let res = match request {
                                0 => {
                                    // Disable remote
                                    shared.enable_remote = false;
                                    let mut dev = handle.async_lock().await.unwrap();
                                    dev.set_local_lockout(false);
                                    dev.set_remote(false)
                                }
                                1 => {
                                    // Enable remote
                                    shared.enable_remote = true;
                                    Ok(())
                                }
                                2 => {
                                    // Disable remote and go to local
                                    shared.enable_remote = false;
                                    let mut dev = handle.async_lock().await.unwrap();
                                    dev.set_local_lockout(false);
                                    dev.set_remote(false)
                                }
                                3 => {
                                    //Enable remote and go to remote
                                    shared.enable_remote = true;
                                    let mut dev = handle.async_lock().await.unwrap();
                                    dev.set_remote(false)
                                }
                                4 => {
                                    // Enable remote and lock out local
                                    shared.enable_remote = true;
                                    let mut dev = handle.async_lock().await.unwrap();
                                    dev.set_local_lockout(true);
                                    Ok(())
                                }
                                5 => {
                                    // Enable remote, got to remote, and set local lockout
                                    shared.enable_remote = true;
                                    let mut dev = handle.async_lock().await.unwrap();
                                    dev.set_local_lockout(true);
                                    dev.set_remote(true)
                                }
                                6 => {
                                    // Go to local without changing state of remote enable
                                    let mut dev = handle.async_lock().await.unwrap();
                                    dev.set_remote(false)
                                }
                                _ => Err(DeviceError::NotSupported),
                            };
                            drop(shared);
                            drop(handle);

                            match res {
                                Ok(_) => {
                                    MessageType::AsyncRemoteLocalResponse
                                        .message_params(0, 0)
                                        .no_payload()
                                        .write_to(&mut wr)
                                        .await?
                                }
                                Err(DeviceError::NotSupported) => {
                                    send_nonfatal!(peer=peer.to_string(), session_id=self.id; &mut wr,
                                        NonFatalErrorCode::UnrecognizedControlCode,
                                        "Unrecognized control code",
                                    );
                                }
                                Err(_) => {
                                    send_nonfatal!(peer=peer.to_string(), session_id=self.id; &mut wr,
                                        NonFatalErrorCode::UnidentifiedError,
                                        "Internal error",
                                    );
                                }
                            }
                        }
                        Message {
                            message_type: MessageType::AsyncMaximumMessageSize,
                            payload,
                            ..
                        } => {
                            if payload.len() != 8 {
                                send_fatal!(peer=peer.to_string(), session_id=self.id;
                                &mut wr, FatalErrorCode::PoorlyFormattedMessageHeader,
                                    "Expected 8 bytes in AsyncMaximumMessageSize payload"
                                )
                            }

                            let size = NetworkEndian::read_u64(payload.as_slice());
                            // Set and quickly release
                            {
                                let mut shared = self.shared.lock().await;
                                shared.max_message_size = size;
                            }
                            log::debug!(peer=peer.to_string(), session_id=self.id; "Max client message size = {}", size);

                            let mut buf = [0u8; 8];

                            NetworkEndian::write_u64(&mut buf, self.config.max_message_size);
                            MessageType::AsyncMaximumMessageSizeResponse
                                .message_params(0, 0)
                                .with_payload(buf.to_vec())
                                .write_to(&mut wr)
                                .await?;
                        }
                        Message {
                            message_type: MessageType::AsyncDeviceClear,
                            ..
                        } => {
                            let shared = self.shared.lock().await;

                            log::debug!(session_id=self.id; "Device clear");

                            // Send a clear event
                            let _ = self.clear.try_send(());

                            // Announce preferred features
                            let features =
                                FeatureBitmap::new(self.config.prefer_overlap, false, false);
                            drop(shared);

                            MessageType::AsyncDeviceClearAcknowledge
                                .message_params(features.0, 0)
                                .no_payload()
                                .write_to(&mut wr)
                                .await?;
                        }
                        Message {
                            message_type: MessageType::AsyncStatusQuery,
                            control_code,
                            message_parameter: message_id,
                            ..
                        } => {
                            let _control = RmtDeliveredControl(control_code);

                            let stb = {
                                let shared = self.shared.lock().await;
                                let handle = self.handle.lock();
                                let mut dev = handle.inner_lock().await;

                                // Calculate MAV bit
                                let sent = shared.sent_message_id;
                                let mav = if sent > message_id { 0x10 } else { 0x00 };

                                // Enable remote
                                if shared.enable_remote {
                                    let _res = dev.set_remote(true);
                                }

                                // Get status of device
                                dev.get_status().unwrap_or(0) & 0xef | mav
                            };

                            srq_bit = false;

                            MessageType::AsyncStatusResponse
                                .message_params(stb, 0)
                                .no_payload()
                                .write_to(&mut wr)
                                .await?;
                        }
                        Message {
                            message_type: MessageType::AsyncLockInfo,
                            ..
                        } => {
                            let (exclusive, num_shared) = {
                                let handle = self.handle.lock();
                                handle.lock_info()
                            };

                            log::debug!(session_id = self.id; "Lock info, exclusive={}, shared={}", exclusive, num_shared);

                            MessageType::AsyncLockInfoResponse
                                .message_params(exclusive.into(), num_shared)
                                .no_payload()
                                .write_to(&mut wr)
                                .await?;
                        }
                        Message {
                            message_type: MessageType::AsyncStartTLS,
                            control_code,
                            message_parameter,
                            payload,
                        } if protocol >= PROTOCOL_2_0 => {
                            if payload.len() != 4 {
                                send_fatal!(peer=peer.to_string(), session_id=self.id;
                                    &mut wr, FatalErrorCode::PoorlyFormattedMessageHeader,
                                    "Expected 4 bytes in AsyncStartTLS payload"
                                )
                            }

                            let _control = RmtDeliveredControl(control_code);
                            let message_id_sent = message_parameter;
                            let message_id_read = NetworkEndian::read_u32(&payload);

                            log::debug!(session_id=self.id, message_id_sent=message_id_sent, message_id_read=message_id_read; "Start async TLS");

                            // TODO: Encryption support
                            send_fatal!(
                                &mut wr,
                                FatalErrorCode::SecureConnectionFailed,
                                "Secure connection not supported"
                            )
                        }
                        Message {
                            message_type: MessageType::AsyncEndTLS,
                            control_code,
                            message_parameter,
                            payload,
                        } if protocol >= PROTOCOL_2_0 => {
                            // Only supported >= 2.0

                            let _control = RmtDeliveredControl(control_code);
                            let message_id_sent = message_parameter;
                            let message_id_read = NetworkEndian::read_u32(&payload);

                            log::debug!(session_id=self.id, message_id_sent=message_id_sent, message_id_read=message_id_read; "Stop async TLS");

                            // TODO: Encryption support
                            send_fatal!(
                                &mut wr,
                                FatalErrorCode::SecureConnectionFailed,
                                "Secure connection not supported"
                            )
                        }
                        _ => {
                            send_nonfatal!(peer=peer.to_string(), session_id=self.id; &mut wr,
                                NonFatalErrorCode::UnrecognizedMessageType,
                                "Unexpected message type in asynchronous channel",
                            );
                        }
                    }
                }
                Err(err) => {
                    // Send error to client and close if fatal
                    if err.is_fatal() {
                        Message::from(err).write_to(&mut wr).await?;
                        break Err(io::ErrorKind::Other.into());
                    } else {
                        Message::from(err).write_to(&mut wr).await?;
                    }
                }
            }
        }
    }
}
