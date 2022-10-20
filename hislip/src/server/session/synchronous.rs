use std::io;
use std::str::from_utf8;
use std::time::Duration;

use async_std::channel::Receiver;
use async_std::future::timeout;
use async_std::sync::Arc;
use futures::lock::Mutex;
use futures::{select, AsyncRead, AsyncWrite, AsyncWriteExt, FutureExt};
use lxi_device::lock::RemoteLockHandle;
use lxi_device::trigger::Source;
use lxi_device::Device;

use crate::common::errors::{Error, FatalErrorCode, NonFatalErrorCode};
use crate::common::messages::{prelude::*, send_fatal, send_nonfatal};
use crate::common::{Protocol, PROTOCOL_2_0};

use super::{ServerConfig, SharedSession};
use crate::server::session::{SessionMode, SessionState};

pub(crate) struct SyncSession<DEV>
where
    DEV: Device,
{
    /// Session ID
    id: u16,

    // Config
    config: ServerConfig,

    /// Shared resources
    handle: RemoteLockHandle<DEV>,

    /// Device
    shared: Arc<Mutex<SharedSession>>,

    clear: Receiver<()>,
}

impl<DEV> SyncSession<DEV>
where
    DEV: Device,
{
    pub(crate) fn new(
        id: u16,
        config: ServerConfig,
        shared: Arc<Mutex<SharedSession>>,
        handle: RemoteLockHandle<DEV>,
        clear: Receiver<()>,
    ) -> Self {
        Self {
            id,
            config,
            shared,
            handle,
            clear,
        }
    }

    async fn acknowledge_device_clear<S>(
        &self,
        mut stream: S,
        peer: String,
        control_code: u8,
    ) -> Result<(), io::Error>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        let mut shared = self.shared.lock().await;
        let feature_request = FeatureBitmap(control_code);
        log::debug!(peer=peer.to_string(), session_id = self.id; "Device clear complete, {}", feature_request);

        shared.set_state(SessionState::Normal);

        // Client might prefer overlapped/synch, fine.
        shared.mode = if feature_request.overlapped() {
            SessionMode::Overlapped
        } else {
            SessionMode::Synchronized
        };

        // Agreed features
        let feature_setting = FeatureBitmap::new(feature_request.overlapped(), false, false);
        let sent_message_id = shared.sent_message_id;
        drop(shared);

        MessageType::DeviceClearAcknowledge
            .message_params(feature_setting.0, sent_message_id)
            .no_payload()
            .write_to(&mut stream)
            .await
    }

    pub(crate) async fn handle_session<S>(
        self,
        mut stream: S,
        peer: String,
        protocol: Protocol,
    ) -> Result<(), io::Error>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        // Data buffer
        let mut buffer: Vec<u8> = Vec::new();

        loop {
            let msg = Message::read_from(&mut stream, self.config.max_message_size).await?;

            // Do not read messages unless a loc
            match msg {
                // Valid message
                Ok(msg) => {
                    match msg {
                        Message {
                            message_type: MessageType::VendorSpecific(code),
                            ..
                        } => {
                            send_nonfatal!(peer=peer.to_string(), session_id=self.id;
                                &mut stream, NonFatalErrorCode::UnrecognizedVendorDefinedMessage,
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
                            message_type: typ @ MessageType::Data | typ @ MessageType::DataEnd,
                            message_parameter: message_id,
                            payload: data,
                            control_code,
                            ..
                        } => {
                            let control = RmtDeliveredControl(control_code);
                            let is_end = matches!(typ, MessageType::DataEnd);

                            // Wait for device becoming available or a lock is acquired
                            // Abort the lock attempt if a clear device is started
                            let mut dev = select! {
                                res = self.handle.async_lock().fuse() => match res{
                                    Ok(res) => res,
                                    Err(_) => {
                                        send_fatal!(peer=peer.to_string(), session_id=self.id;
                                            &mut stream,
                                            FatalErrorCode::UnidentifiedError,
                                            "Internal locking error"
                                        );
                                    }
                                },
                                _abort = self.clear.recv().fuse() => {
                                    buffer.clear();
                                    self.acknowledge_device_clear(&mut stream, peer.clone(), control_code).await?;
                                    continue;
                                },
                            };

                            let mut shared = self.shared.lock().await;
                            let state = shared.state();

                            match state {
                                // Normal state
                                SessionState::Normal => {
                                    shared.read_message_id = message_id;
                                    drop(shared); // Drop shared data as to not block async session

                                    if buffer.try_reserve_exact(data.len()).is_err() {
                                        send_fatal!(peer=peer.to_string(), session_id=self.id;
                                            &mut stream,
                                            FatalErrorCode::UnidentifiedError,
                                            "Out of memory"
                                        );
                                    }
                                    buffer.extend_from_slice(&data);
                                    log::info!("Buffer={:?}", data);

                                    if is_end {
                                        log::debug!(peer=peer.to_string(), session_id=self.id, message_id=message_id; "Data END, {}", control);

                                        // TODO: Replace with .trim_ascii_end() when available
                                        let data = if buffer.eq_ignore_ascii_case(b"*idn?")
                                            || buffer.eq_ignore_ascii_case(b"*idn?\n")
                                                && self.config.short_idn.is_some()
                                        {
                                            self.config.short_idn.clone()
                                        } else {
                                            let data = dev.execute(&buffer);
                                            data
                                        };
                                        buffer.clear();

                                        // Send back response
                                        let shared = self.shared.lock().await;
                                        if let Some(data) = data {
                                            log::info!("Sending back");
                                            let mut chunks = data
                                                .chunks(shared.max_message_size as usize)
                                                .peekable();
                                            drop(shared);

                                            while let Some(chunk) = chunks.next() {
                                                // Stop sending if a clear has been received on async channel
                                                if self.clear.try_recv().is_ok() {
                                                    log::info!("Sending back, clear!");

                                                    break;
                                                }

                                                // Peek if next chunk exists, if not, mark data as end
                                                let end = chunks.peek().is_none();
                                                let msg = if end {
                                                    MessageType::DataEnd
                                                } else {
                                                    MessageType::Data
                                                };

                                                // Send message
                                                msg.message_params(0, message_id)
                                                    .with_payload(chunk.to_vec())
                                                    .write_to(&mut stream)
                                                    .await?;
                                            }
                                        }
                                    } else {
                                        log::debug!(peer=peer.to_string(), session_id=self.id, message_id=message_id; "Data, {}", control);
                                    }

                                    // Do not acknowledge
                                }
                                // Initial handshake
                                SessionState::Handshake => {
                                    send_fatal!(peer=peer.to_string(), session_id=self.id;
                                        &mut stream,
                                        FatalErrorCode::AttemptUseWithoutBothChannels,
                                        "Attempted use without both channels"
                                    );
                                }
                            }
                        }
                        Message {
                            message_type: MessageType::Trigger,
                            message_parameter: message_id,
                            control_code,
                            ..
                        } => {
                            // Wait for device becoming available or a lock is acquired
                            // Abort the lock attempt if a clear device is started
                            let mut dev = select! {
                                res = self.handle.async_lock().fuse() => match res{
                                    Ok(res) => res,
                                    Err(_) => {
                                        send_fatal!(peer=peer.to_string(), session_id=self.id;
                                            &mut stream,
                                            FatalErrorCode::UnidentifiedError,
                                            "Internal locking error"
                                        );
                                    }
                                },
                                _abort = self.clear.recv().fuse() => {
                                    buffer.clear();
                                    self.acknowledge_device_clear(&mut stream, peer.clone(), control_code).await?;
                                    continue;
                                }
                            };

                            let mut inner = self.shared.lock().await;
                            inner.read_message_id = message_id;
                            let state = inner.state();
                            drop(inner);

                            match state {
                                SessionState::Normal => {
                                    let control = RmtDeliveredControl(control_code);
                                    log::debug!(session_id=self.id, message_id=message_id; "Trigger, {}", control);

                                    let _ = dev.trigger(Source::Bus);
                                }
                                // Initial handshake
                                SessionState::Handshake => {
                                    send_fatal!(peer=peer.to_string(), session_id=self.id;
                                        &mut stream,
                                        FatalErrorCode::AttemptUseWithoutBothChannels,
                                        "Attempted use without both channels"
                                    );
                                }
                            }
                        }
                        Message {
                            message_type: MessageType::DeviceClearComplete,
                            control_code,
                            ..
                        } => match timeout(Duration::from_secs(10), self.clear.recv()).await {
                            Ok(Ok(())) => {
                                buffer.clear();
                                self.acknowledge_device_clear(
                                    &mut stream,
                                    peer.clone(),
                                    control_code,
                                )
                                .await?;
                            }
                            Ok(Err(_rerr)) => {
                                send_fatal!(peer=peer.to_string(), session_id=self.id;
                                    &mut stream,
                                    FatalErrorCode::UnidentifiedError,
                                    "Internal server error"
                                );
                            }
                            Err(_terr) => {
                                send_fatal!(peer=peer.to_string(), session_id=self.id;
                                    &mut stream,
                                    FatalErrorCode::UnidentifiedError,
                                    "Received device clear complete without a request"
                                );
                            }
                        },
                        Message {
                            message_type: MessageType::GetDescriptors,
                            ..
                        } if protocol >= PROTOCOL_2_0 => {
                            todo!()
                        }
                        Message {
                            message_type: MessageType::StartTLS | MessageType::EndTLS,
                            ..
                        } if protocol >= PROTOCOL_2_0 => {
                            log::debug!(peer=peer.to_string(), session_id=self.id; "Start/end TLS");

                            send_fatal!(
                                &mut stream,
                                FatalErrorCode::SecureConnectionFailed,
                                "Secure connection not supported"
                            )
                        }
                        Message {
                            message_type:
                                MessageType::GetSaslMechanismList
                                | MessageType::AuthenticationStart
                                | MessageType::AuthenticationExchange,
                            payload: _data,
                            ..
                        } if protocol >= PROTOCOL_2_0 => {
                            log::debug!(peer=peer.to_string(), session_id=self.id; "Authentication Start/Exchange");

                            send_fatal!(
                                &mut stream,
                                FatalErrorCode::SecureConnectionFailed,
                                "Authentication not supported"
                            )
                        }
                        msg => {
                            send_nonfatal!(peer=peer.to_string(), session_id=self.id;
                                &mut stream,
                                NonFatalErrorCode::UnidentifiedError,
                                "Unexpected message type in synchronous channel: {:?}", msg.message_type
                            );
                        }
                    }
                }
                // Invalid message
                Err(err) => {
                    if err.is_fatal() {
                        Message::from(err).write_to(&mut stream).await?;
                        return Err(io::ErrorKind::Other.into());
                    } else {
                        Message::from(err).write_to(&mut stream).await?;
                    }
                }
            }
        }
    }
}
