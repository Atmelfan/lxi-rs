use std::io;
use std::net::SocketAddr;
use std::str::from_utf8;

use async_std::channel::Receiver;
use async_std::sync::Arc;
use futures::lock::Mutex;
use futures::{select, AsyncWriteExt, FutureExt};
use lxi_device::lock::RemoteLockHandle;
use lxi_device::Device;
use lxi_device::trigger::Source;
use sasl::server::Mechanism;

use crate::common::errors::{Error, FatalErrorCode, NonFatalErrorCode};
use crate::common::messages::{prelude::*, send_fatal, send_nonfatal};
use crate::common::PROTOCOL_2_0;

use super::{ServerConfig, SharedSession};
use crate::server::auth::{Auth, SaslResponse};
use crate::server::session::{SessionMode, SessionState};
use crate::server::stream::HislipStream;

pub(crate) struct SyncSession<DEV, A>
where
    DEV: Device,
    A: Auth + Send + 'static,
{
    /// Session ID
    id: u16,

    // Config
    config: ServerConfig,

    /// Shared resources
    handle: RemoteLockHandle<DEV>,

    /// Device
    shared: Arc<Mutex<SharedSession>>,

    ///
    event: Receiver<()>,

    // Authentications
    authenticator: Arc<Mutex<A>>,
}

impl<DEV, A> SyncSession<DEV, A>
where
    DEV: Device,
    A: Auth + Send + 'static,
{
    pub(crate) async fn new(
        id: u16,
        config: ServerConfig,
        shared: Arc<Mutex<SharedSession>>,
        handle: RemoteLockHandle<DEV>,
        authenticator: Arc<Mutex<A>>,
    ) -> Self {
        let event = shared.lock().await.clear.1.clone();
        Self {
            id,
            config,
            shared,
            handle,
            event,
            authenticator,
        }
    }

    async fn acknowledge_device_clear(
        &self,
        stream: &mut HislipStream<'_>,
        peer: SocketAddr,
        control_code: u8,
    ) -> Result<(), io::Error> {
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
        let feature_setting = FeatureBitmap::new(
            feature_request.overlapped(),
            self.config.encryption_mode && shared.protocol >= PROTOCOL_2_0,
            self.config.initial_encryption && shared.protocol >= PROTOCOL_2_0,
        );
        let sent_message_id = shared.sent_message_id;
        drop(shared);

        MessageType::DeviceClearAcknowledge
            .message_params(feature_setting.0, sent_message_id)
            .no_payload()
            .write_to(stream)
            .await
    }

    async fn clear_buffer(
        &self,
        stream: &mut HislipStream<'_>,
        peer: SocketAddr,
        mut msg: Result<Message, Error>,
    ) -> Result<(), io::Error> {
        loop {
            match msg {
                Ok(Message {
                    message_type: MessageType::DeviceClearComplete,
                    control_code,
                    ..
                }) => {
                    if self.handle.can_lock().is_ok() {
                        let mut dev = self.handle.inner_lock().await;
                        let _res = dev.clear();
                    }

                    break self
                        .acknowledge_device_clear(stream, peer, control_code)
                        .await;
                }
                // Ignore other messages
                Ok(_) => {}
                // Invalid message
                Err(err) => {
                    if err.is_fatal() {
                        Message::from(err).write_to(stream).await?;
                        return Err(io::ErrorKind::Other.into());
                    } else {
                        Message::from(err).write_to(stream).await?;
                    }
                }
            }
            msg = Message::read_from(stream, self.config.max_message_size).await?;
        }
    }

    pub(crate) async fn handle_session(
        self,
        mut stream: HislipStream<'_>,
        peer: SocketAddr,
        #[cfg(feature = "tls")] acceptor: async_tls::TlsAcceptor,
    ) -> Result<(), io::Error> {
        // Data buffer
        let mut buffer: Vec<u8> = Vec::new();

        // Current authentication mechanim
        let mut mechanism: Option<Box<dyn Mechanism + Send>> = None;

        loop {
            let msg = Message::read_from(&mut stream, self.config.max_message_size).await?;

            // Check if a clear device is in progress before waiting for a lock
            if let Ok(_abort) = self.event.try_recv() {
                // Clear buffer
                buffer.clear();
                self.clear_buffer(&mut stream, peer.clone(), msg).await?;
                continue;
            }

            // Wait for device becoming available or a lock is acquired
            // Abort the lock attempt if a clear device is started
            let mut dev = select! {
                res = self.handle.async_lock().fuse() => res.unwrap(),
                _abort = self.event.recv().fuse() => {
                    // Clear buffer
                    buffer.clear();
                    self.clear_buffer(&mut stream, peer.clone(), msg).await?;
                    continue;
                }
            };

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
                            message_type: MessageType::FatalError,
                            control_code,
                            payload,
                            ..
                        } => {
                            log::error!(peer=peer.to_string(), session_id=self.id;
                                "Client fatal error {:?}: {}", FatalErrorCode::from_error_code(control_code),
                                from_utf8(&payload).unwrap_or("<invalid utf8>")
                            );
                        }
                        Message {
                            message_type: MessageType::Error,
                            control_code,
                            payload,
                            ..
                        } => {
                            log::warn!(peer=peer.to_string(), session_id=self.id;
                                "Client error {:?}: {}", NonFatalErrorCode::from_error_code(control_code),
                                from_utf8(&payload).unwrap_or("<invalid utf8>")
                            );
                        }
                        Message {
                            message_type: typ @ MessageType::Data | typ @ MessageType::DataEnd,
                            message_parameter: message_id,
                            payload: data,
                            ..
                        } => {
                            let is_end = matches!(typ, MessageType::DataEnd);

                            let mut shared = self.shared.lock().await;
                            let state = shared.state();

                            match state {
                                // Normal state
                                SessionState::Normal => {
                                    shared.read_message_id = message_id;
                                    buffer.extend_from_slice(&data);

                                    if !is_end {
                                        log::debug!(peer=peer.to_string(), session_id=self.id, message_id=message_id; "Data");
                                    } else {
                                        log::debug!(peer=peer.to_string(), session_id=self.id, message_id=message_id; "Data END");
                                        let data = dev.execute(&buffer);

                                        let mut chunks = data
                                            .chunks(shared.max_message_size as usize)
                                            .peekable();
                                        drop(shared);

                                        while let Some(chunk) = chunks.next() {
                                            // Stop sending if a clear has been sent
                                            if self.event.try_recv().is_ok() {
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

                                        log::trace!("Write")
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
                                // Othe secure connection states
                                SessionState::EncryptionStart
                                | SessionState::AuthenticationExchange
                                | SessionState::AuthenticationStart
                                | SessionState::EncryptionStop => {
                                    send_fatal!(peer=peer.to_string(), session_id=self.id;
                                        &mut stream,
                                        FatalErrorCode::SecureConnectionFailed,
                                        "Unexpected message during establishment of secure connection"
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
                                // Othe secure connection states
                                SessionState::EncryptionStart
                                | SessionState::AuthenticationExchange
                                | SessionState::AuthenticationStart
                                | SessionState::EncryptionStop => {
                                    send_fatal!(peer=peer.to_string(), session_id=self.id;
                                        &mut stream,
                                        FatalErrorCode::SecureConnectionFailed,
                                        "Unexpected message during establishment of secure connection"
                                    );
                                }
                            }
                        }
                        Message {
                            message_type: MessageType::DeviceClearComplete,
                            ..
                        } => {
                            // Should've been handled above when AsyncDeviceClear was sent
                            send_nonfatal!(peer=peer.to_string(), session_id=self.id;
                                &mut stream,
                                NonFatalErrorCode::UnidentifiedError,
                                "Unexpected device clear complete in synchronous channel"
                            );
                        }
                        Message {
                            message_type: MessageType::GetDescriptors,
                            ..
                        } => {}
                        Message {
                            message_type: MessageType::StartTLS,
                            ..
                        } => {
                            let mut shared = self.shared.lock().await;

                            log::debug!(peer=peer.to_string(), session_id=self.id; "Start sync TLS");

                            // Why did you send this?
                            if shared.protocol() < PROTOCOL_2_0 {
                                send_nonfatal!(peer=peer.to_string(), session_id=self.id;
                                    &mut stream,
                                    NonFatalErrorCode::UnrecognizedMessageType,
                                    "Negotiated protocol version does not support this message"
                                );
                                continue;
                            }

                            match shared.state() {
                                SessionState::Normal | SessionState::EncryptionStart => {
                                    #[cfg(feature = "tls")]
                                    match stream.start_tls(acceptor.clone()).await {
                                        Ok(_) => shared.set_state(SessionState::AuthenticationStart),
                                        Err(_) => send_fatal!(
                                            &mut stream,
                                            FatalErrorCode::SecureConnectionFailed,
                                            "Failed to start encryption"
                                        ),
                                    }

                                    #[cfg(not(feature = "tls"))]
                                    send_fatal!(
                                        &mut stream,
                                        FatalErrorCode::SecureConnectionFailed,
                                        "Secure connection not supported"
                                    )
                                }
                                SessionState::Handshake => {
                                    send_fatal!(peer=peer.to_string(), session_id=self.id;
                                        &mut stream,
                                        FatalErrorCode::AttemptUseWithoutBothChannels,
                                        "Attempted use without both channels"
                                    );
                                }
                                SessionState::AuthenticationExchange
                                | SessionState::AuthenticationStart
                                | SessionState::EncryptionStop => {
                                    send_fatal!(peer=peer.to_string(), session_id=self.id;
                                        &mut stream,
                                        FatalErrorCode::SecureConnectionFailed,
                                        "Unexpected message during establishment of secure connection"
                                    );
                                }
                            }
                        }
                        Message {
                            message_type: MessageType::GetSaslMechanismList,
                            ..
                        } => {
                            let auth = self.authenticator.lock().await;
                            let shared = self.shared.lock().await;

                            // Why did you send this?
                            if shared.protocol() < PROTOCOL_2_0 {
                                send_nonfatal!(peer=peer.to_string(), session_id=self.id;
                                    &mut stream,
                                    NonFatalErrorCode::UnrecognizedMessageType,
                                    "Negotioted protocol version does not support this message"
                                );
                                continue;
                            }

                            let supported = auth.list_mechanisms();
                            let resp = supported.join(" ");

                            MessageType::GetSaslMechanismListResponse
                                .message_params(0, 0)
                                .with_payload(resp.into_bytes())
                                .write_to(&mut stream)
                                .await?;
                        }
                        Message {
                            message_type: MessageType::AuthenticationStart,
                            payload: data,
                            ..
                        } => {
                            let auth = self.authenticator.lock().await;
                            let mut shared = self.shared.lock().await;

                            // Why did you send this?
                            if shared.protocol() < PROTOCOL_2_0 {
                                send_nonfatal!(peer=peer.to_string(), session_id=self.id;
                                    &mut stream,
                                    NonFatalErrorCode::UnrecognizedMessageType,
                                    "Negotioted protocol version does not support this message"
                                );
                                continue;
                            }

                            // Only allowed after AuthenticationStart
                            if matches!(shared.state(), SessionState::AuthenticationStart) {
                                // Initialize the machanism
                                let res = if let Ok(name) = from_utf8(&data) {
                                    auth.start_exchange(name).map_err(|err| err.into())
                                } else {
                                    Err(Error::NonFatal(
                                        NonFatalErrorCode::AuthenticationFailed,
                                        "Mechanism name is not valid UTF8".to_string(),
                                    ))
                                };

                                // Ok (don't acknowledge) or error
                                match res {
                                    Ok(mech) => {
                                        let _ = mechanism.replace(mech);
                                        shared.set_state(SessionState::AuthenticationExchange);
                                    }
                                    Err(err) => Message::from(err).write_to(&mut stream).await?,
                                }
                            } else {
                                send_nonfatal!(peer=peer.to_string(), session_id=self.id;
                                    &mut stream,
                                    NonFatalErrorCode::UnidentifiedError,
                                    "Unexpected authentication start"
                                );
                            }
                        }
                        Message {
                            message_type: MessageType::AuthenticationExchange,
                            payload: data,
                            ..
                        } => {
                            let mut shared = self.shared.lock().await;

                            // Why did you send this?
                            if shared.protocol() < PROTOCOL_2_0 {
                                send_nonfatal!(peer=peer.to_string(), session_id=self.id;
                                    &mut stream,
                                    NonFatalErrorCode::UnrecognizedMessageType,
                                    "Negotioted protocol version does not support this message"
                                );
                                continue;
                            }

                            // Only allowed after AuthenticationStart
                            if matches!(shared.state(), SessionState::AuthenticationExchange) {
                                if let Some(mech) = &mut mechanism {
                                    match mech.respond(&data) {
                                        Ok(SaslResponse::Proceed(data)) => {
                                            // Send an exchange
                                            MessageType::AuthenticationExchange
                                                .message_params(0, 0)
                                                .with_payload(data)
                                                .write_to(&mut stream)
                                                .await?;
                                        }
                                        Ok(SaslResponse::Success(id, data)) => {
                                            // Clear mechanism
                                            let _ = mechanism.take();

                                            let id = match id {
                                                sasl::common::Identity::None => {
                                                    "<Anonymous>".to_string()
                                                }
                                                sasl::common::Identity::Username(name) => name,
                                            };
                                            log::info!(peer=peer.to_string(), session_id=self.id; "Authenticated as {id}");

                                            // Send result
                                            MessageType::AuthenticationResult
                                                .message_params(1, 0)
                                                .with_payload(data)
                                                .write_to(&mut stream)
                                                .await?;

                                            // Done, go to normal state
                                            shared.set_state(SessionState::Normal);
                                        }
                                        Err(err) => {
                                            let msg = format!("{}", err);

                                            // Send result
                                            MessageType::AuthenticationResult
                                                .message_params(0, 0)
                                                .with_payload(msg.into_bytes())
                                                .write_to(&mut stream)
                                                .await?;

                                            // Error, go back to start of authentication
                                            shared.set_state(SessionState::AuthenticationStart);
                                        }
                                    }
                                }
                            } else {
                                send_nonfatal!(peer=peer.to_string(), session_id=self.id;
                                    &mut stream,
                                    NonFatalErrorCode::UnidentifiedError,
                                    "Unexpected authentication exchange"
                                );
                            }
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
