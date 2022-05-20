use std::io;
use std::net::SocketAddr;
use std::str::from_utf8;
use std::time::Duration;

use async_std::channel::{Receiver, Sender};
use async_std::future;
use async_std::net::TcpStream;
use async_std::sync::Arc;
use byteorder::{ByteOrder, NetworkEndian};
use futures::lock::Mutex;
use futures::{select, FutureExt, StreamExt};
use lxi_device::lock::{LockHandle, SharedLockError, SharedLockMode};
use lxi_device::{Device, DeviceError};

use crate::common::errors::{Error, FatalErrorCode, NonFatalErrorCode};
use crate::common::messages::{prelude::*, send_fatal, send_nonfatal};
use crate::common::{Protocol, PROTOCOL_2_0};

use crate::server::auth::Auth;
use super::stream::{HislipStream, HISLIP_TLS_BUSY, HISLIP_TLS_ERROR, HISLIP_TLS_SUCCESS};
use super::ServerConfig;

#[cfg(feature = "tls")]
use async_tls::TlsAcceptor;
#[cfg(feature = "tls")]
use sasl::{
    secret,
    server::{
        mechanisms::{Anonymous, Plain},
        Validator,
    },
};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SessionMode {
    Synchronized,
    Overlapped,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub(crate) enum SessionState {
    Handshake,
    EncryptionStart,
    AuthenticationStart,
    Normal,
    Clear,
}

macro_rules! device_remote {
    ($self:expr, $dev:expr) => {
        if $self.enable_remote {
            let _ = $dev.set_remote(true);
        }
    };
}

pub(crate) struct Session<DEV>
where
    DEV: Device,
{
    config: ServerConfig,
    /// Negotiated rpc
    protocol: Protocol,
    /// Current tate of session
    state: SessionState,

    /// Negotiated session mode
    pub(crate) mode: SessionMode,
    /// Session ID
    pub(crate) id: u16,
    /// Client max message size
    pub(crate) max_message_size: u64,

    pub(crate) handle: LockHandle<DEV>,

    // Input/Output buffer
    pub(crate) in_buf: Vec<u8>,
    pub(crate) out_buf: Vec<u8>,

    sent_message_id: u32,
    read_message_id: u32,

    //
    enable_remote: bool,
}

impl<DEV> Session<DEV>
where
    DEV: Device,
{
    pub(crate) fn new(
        config: ServerConfig,
        session_id: u16,
        protocol: Protocol,
        handle: LockHandle<DEV>,
    ) -> Self {
        Self {
            config,
            protocol,
            mode: SessionMode::Overlapped,
            id: session_id,
            max_message_size: 256,
            handle,
            in_buf: Vec::new(),
            out_buf: Vec::new(),
            enable_remote: true,
            sent_message_id: 0xffff_ff00,
            read_message_id: 0xffff_ff00,

            state: SessionState::Handshake,
        }
    }

    pub(crate) fn close(&mut self) {
        log::debug!(session_id=self.id; "Closing session");
        // Release any lock this session might be holding
        // Should be called anyways by LockHandle::drop() but done here to be obvious
        self.handle.force_release();
    }

    pub(crate) async fn handle_sync_session(
        session: Arc<Mutex<Self>>,
        stream: &mut HislipStream<'_>,
        peer: SocketAddr,
        config: ServerConfig,
        #[cfg(feature = "tls")] acceptor: TlsAcceptor,
    ) -> Result<(), io::Error> {
        loop {
            match Message::read_from(stream, config.max_message_size).await? {
                // Valid message
                Ok(msg) => {
                    let mut session = session.lock().await;
                    session
                        .handle_sync_message(
                            msg,
                            stream,
                            peer,
                            #[cfg(feature = "tls")]
                            acceptor.clone(),
                        )
                        .await?;
                }
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
        }
    }

    pub(crate) async fn handle_sync_message(
        self: &mut Self,
        msg: Message,
        stream: &mut HislipStream<'_>,
        peer: SocketAddr,
        #[cfg(feature = "tls")] acceptor: TlsAcceptor,
    ) -> Result<(), io::Error> {
        // Still handshaking
        if let SessionState::Handshake = self.state {
            send_fatal!(peer=peer.to_string(), session_id=self.id;
                stream,
                FatalErrorCode::AttemptUseWithoutBothChannels,
                "Attempted to use without both channels"
            )
        }

        match msg {
            Message {
                message_type: MessageType::VendorSpecific(code),
                ..
            } => {
                send_nonfatal!(peer=peer.to_string(), session_id=self.id;
                    stream, NonFatalErrorCode::UnrecognizedVendorDefinedMessage,
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
                payload,
                ..
            } => {
                self.read_message_id = message_id;
                match self.state {
                    // Put data in buffer and possibly execute
                    SessionState::Normal => {
                        let mut dev = self.handle.async_lock().await.unwrap();
                        device_remote!(self, dev);

                        // Save data in buffer
                        self.in_buf.extend_from_slice(&payload);
                        log::trace!(session_id=self.id, message_id=message_id; "Data {:?}", payload);

                        // Execute if END is implied
                        if typ == MessageType::DataEnd {
                            let out = dev.execute(&self.in_buf);
                            self.in_buf.clear();

                            // Send back any output data
                            if !out.is_empty() {
                                let mut iter = out.chunks_exact(self.max_message_size as usize);
                                while let Some(chunk) = iter.next() {
                                    MessageType::Data
                                        .message_params(0, self.sent_message_id)
                                        .with_payload(chunk.to_vec())
                                        .write_to(stream)
                                        .await?;
                                    self.sent_message_id = self.sent_message_id.wrapping_add(2);
                                }
                                MessageType::DataEnd
                                    .message_params(0, self.sent_message_id)
                                    .with_payload(iter.remainder().to_vec())
                                    .write_to(stream)
                                    .await?;
                                self.sent_message_id = self.sent_message_id.wrapping_add(2);
                            }
                        }
                    }
                    // Ignore message
                    SessionState::Clear => return Ok(()),
                    // Currently establishing secure connection
                    SessionState::EncryptionStart | SessionState::AuthenticationStart => {
                        send_fatal!(peer=peer.to_string(), session_id=self.id;
                            stream,
                            FatalErrorCode::SecureConnectionFailed,
                            "Unexpected message during secure connection"
                        )
                    }
                    // Still handshaking
                    SessionState::Handshake => {}
                }
            }
            Message {
                message_type: MessageType::Trigger,
                message_parameter: message_id,
                control_code,
                ..
            } => {
                self.read_message_id = message_id;
                match self.state {
                    // Put data in buffer and possibly execute
                    SessionState::Normal => {
                        let control = RmtDeliveredControl(control_code);
                        log::debug!(session_id=self.id, message_id=message_id; "Trigger, {}", control);

                        let mut dev = self.handle.async_lock().await.unwrap();
                        device_remote!(self, dev);
                        let _ = dev.trigger();
                    }
                    // Ignore message
                    SessionState::Clear => return Ok(()),
                    // Currently establishing secure connection
                    SessionState::EncryptionStart | SessionState::AuthenticationStart => {
                        send_fatal!(peer=peer.to_string(), session_id=self.id;
                            stream,
                            FatalErrorCode::SecureConnectionFailed,
                            "Unexpected message during handshake"
                        )
                    }
                    // Still handshaking
                    SessionState::Handshake => {}
                }
            }
            Message {
                message_type: MessageType::DeviceClearComplete,
                control_code,
                ..
            } => {
                match self.state {
                    // Ignore message
                    SessionState::Clear => {
                        let feature_request = FeatureBitmap(control_code);
                        log::debug!(session_id = self.id; "Device clear complete, {}", feature_request);

                        self.state = SessionState::Normal;

                        // Client might prefer overlapped/synch, fine.
                        self.mode = if feature_request.overlapped() {
                            SessionMode::Overlapped
                        } else {
                            SessionMode::Synchronized
                        };

                        // Agreed features
                        let feature_setting = FeatureBitmap::new(
                            feature_request.overlapped(),
                            self.config.encryption_mode && self.protocol >= PROTOCOL_2_0,
                            self.config.initial_encryption && self.protocol >= PROTOCOL_2_0,
                        );
                        MessageType::DeviceClearAcknowledge
                            .message_params(feature_setting.0, self.sent_message_id)
                            .no_payload()
                            .write_to(stream)
                            .await?;
                    }
                    // Currently doing something else
                    _ => {
                        send_nonfatal!(peer=peer.to_string(), session_id=self.id;
                            stream, NonFatalErrorCode::UnidentifiedError,
                            "Unexpected device clear complete"
                        )
                    }
                }
            }
            Message {
                message_type: MessageType::GetDescriptors,
                ..
            } => {}
            Message {
                message_type: MessageType::StartTLS,
                ..
            } => {}
            Message {
                message_type: MessageType::GetSaslMechanismList,
                ..
            } => {
                let mut supported = "PLAIN ANONYMOUS";

                MessageType::GetSaslMechanismListResponse
                    .message_params(0, 0)
                    .no_payload()
                    .write_to(stream)
                    .await?;
            }
            Message {
                message_type: MessageType::AuthenticationStart,
                ..
            } => {}
            Message {
                message_type: MessageType::AuthenticationExchange,
                ..
            } => {}
            msg => {
                send_nonfatal!(peer=peer.to_string(), session_id=self.id;
                    stream,
                    NonFatalErrorCode::UnidentifiedError,
                    "Unexpected message type in synchronous channel: {:?}", msg.message_type
                );
            }
        }
        Ok(())
    }

    pub(crate) async fn handle_async_session(
        session: Arc<Mutex<Self>>,
        stream: &mut HislipStream<'_>,
        peer: SocketAddr,
        config: ServerConfig,
        #[cfg(feature = "tls")] acceptor: TlsAcceptor,
    ) -> Result<(), io::Error> {
        loop {
            match Message::read_from(stream, config.max_message_size).await? {
                Ok(msg) => {
                    let mut session = session.lock().await;
                    session
                        .handle_async_message(
                            msg,
                            stream,
                            peer,
                            #[cfg(feature = "tls")]
                            acceptor.clone(),
                        )
                        .await?;
                }
                Err(err) => {
                    // Send error to client and close if fatal
                    if err.is_fatal() {
                        Message::from(err).write_to(stream).await?;
                        break Err(io::ErrorKind::Other.into());
                    } else {
                        Message::from(err).write_to(stream).await?;
                    }
                }
            }
        }
    }

    pub(crate) async fn handle_async_message(
        self: &mut Self,
        msg: Message,
        stream: &mut HislipStream<'_>,
        peer: SocketAddr,
        #[cfg(feature = "tls")] acceptor: TlsAcceptor,
    ) -> Result<(), io::Error> {
        match msg {
            Message {
                message_type: MessageType::VendorSpecific(code),
                ..
            } => {
                send_nonfatal!(peer=peer.to_string(), session_id=self.id;
                    stream, NonFatalErrorCode::UnrecognizedVendorDefinedMessage,
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
                //break; // Let client close connection
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
                message_type: MessageType::AsyncLock,
                message_parameter,
                control_code,
                payload: lockstr,
            } => {
                if control_code == 0 {
                    // Release
                    let message_id = message_parameter;
                    log::debug!(peer=peer.to_string(), session_id=self.id, message_id=message_id; "Release async lock");
                    let control = match self.handle.try_release() {
                        Ok(SharedLockMode::Exclusive) => ReleaseLockControl::SuccessExclusive,
                        Ok(SharedLockMode::Shared) => ReleaseLockControl::SuccessShared,
                        Err(_) => ReleaseLockControl::Error,
                    };
                    MessageType::AsyncLockResponse
                        .message_params(control as u8, 0)
                        .no_payload()
                        .write_to(stream)
                        .await?;
                } else {
                    // Lock
                    let timeout = message_parameter;

                    let control = match from_utf8(&lockstr) {
                        Ok(lockstr) => {
                            log::debug!(peer=peer.to_string(), session_id=self.id, timeout=timeout; "Async lock: {:?}", lockstr);
                            // Try to acquire lock
                            let res = if timeout == 0 {
                                // Try to lock immediately
                                self.handle.try_acquire(lockstr.as_bytes())
                            } else {
                                // Try to lock until timed out
                                future::timeout(
                                    Duration::from_millis(timeout as u64),
                                    self.handle.async_acquire(lockstr.as_bytes()),
                                )
                                .await
                                .map_err(|_| SharedLockError::Timeout)
                                .and_then(|res| res)
                            };

                            //log::debug!(session_id = self.id; "Async lock: {:?}", res);

                            match res {
                                Ok(()) => RequestLockControl::Success,
                                Err(SharedLockError::Timeout) => RequestLockControl::Failure,
                                Err(_) => RequestLockControl::Error,
                            }
                        }
                        Err(s) => {
                            log::error!(peer=peer.to_string(), session_id=self.id; "Async lock string is not valid");
                            RequestLockControl::Error
                        }
                    };

                    MessageType::AsyncLockResponse
                        .message_params(control as u8, 0)
                        .no_payload()
                        .write_to(stream)
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
                let mut dev = self.handle.async_lock().await.unwrap();
                let res = match request {
                    0 => {
                        // Disable remote
                        self.enable_remote = false;
                        dev.set_local_lockout(false);
                        dev.set_remote(false)
                    }
                    1 => {
                        // Enable remote
                        self.enable_remote = true;
                        Ok(())
                    }
                    2 => {
                        // Disable remote and go to local
                        self.enable_remote = false;
                        dev.set_local_lockout(false);
                        dev.set_remote(false)
                    }
                    3 => {
                        //Enable remote and go to remote
                        self.enable_remote = true;
                        dev.set_remote(false)
                    }
                    4 => {
                        // Enable remote and lock out local
                        self.enable_remote = true;
                        dev.set_local_lockout(true);
                        Ok(())
                    }
                    5 => {
                        // Enable remote, got to remote, and set local lockout
                        self.enable_remote = true;
                        dev.set_local_lockout(true);
                        dev.set_remote(true)
                    }
                    6 => {
                        // Go to local without changing state of remote enable
                        dev.set_remote(false)
                    }
                    _ => Err(DeviceError::NotSupported),
                };
                match res {
                    Ok(_) => {
                        MessageType::AsyncRemoteLocalResponse
                            .message_params(0, 0)
                            .no_payload()
                            .write_to(stream)
                            .await?
                    }
                    Err(DeviceError::NotSupported) => {
                        send_nonfatal!(peer=peer.to_string(), session_id=self.id; stream,
                            NonFatalErrorCode::UnrecognizedControlCode,
                            "Unrecognized control code",
                        );
                    }
                    Err(_) => {
                        send_nonfatal!(peer=peer.to_string(), session_id=self.id; stream,
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
                        stream, FatalErrorCode::PoorlyFormattedMessageHeader,
                        "Expected 8 bytes in AsyncMaximumMessageSize payload"
                    )
                }

                let size = NetworkEndian::read_u64(payload.as_slice());
                self.max_message_size = size;
                log::debug!(peer=peer.to_string(), session_id=self.id; "Max client message size = {}", size);

                let mut buf = [0u8; 8];

                NetworkEndian::write_u64(&mut buf, self.config.max_message_size as u64);
                MessageType::AsyncMaximumMessageSizeResponse
                    .message_params(0, 0)
                    .with_payload(buf.to_vec())
                    .write_to(stream)
                    .await?;
            }
            Message {
                message_type: MessageType::AsyncDeviceClear,
                ..
            } => {
                let mut dev = self.handle.async_lock().await.unwrap();
                log::debug!(session_id=self.id; "Device clear");

                //
                if self.enable_remote {
                    dev.set_remote(true);
                }

                // TODO: this should abort any in-progress operations
                if let Ok(mut dev) = self.handle.try_lock() {
                    let _ = dev.clear();
                }
                self.state = SessionState::Clear;
                self.in_buf.clear();
                self.out_buf.clear();
                self.sent_message_id = 0xffff_ff00;

                // Announce preferred features
                let features = FeatureBitmap::new(
                    self.config.prefer_overlap,
                    self.config.encryption_mode && self.protocol >= PROTOCOL_2_0,
                    self.config.initial_encryption && self.protocol >= PROTOCOL_2_0,
                );
                MessageType::AsyncDeviceClearAcknowledge
                    .message_params(features.0, 0)
                    .no_payload()
                    .write_to(stream)
                    .await?;
            }
            Message {
                message_type: MessageType::AsyncStatusQuery,
                control_code,
                message_parameter: message_id,
                ..
            } => {
                let stb = {
                    let mut dev = self.handle.async_lock().await.unwrap();

                    //
                    if self.enable_remote {
                        dev.set_remote(true);
                    }

                    dev.get_status().unwrap()
                };

                MessageType::AsyncStatusResponse
                    .message_params(stb, 0)
                    .no_payload()
                    .write_to(stream)
                    .await?;
            }
            Message {
                message_type: MessageType::AsyncLockInfo,
                ..
            } => {
                let (exclusive, num_shared) = self.handle.lock_info();

                log::debug!(session_id = self.id; "Lock info, exclusive={}, shared={}", exclusive, num_shared);

                MessageType::AsyncLockInfoResponse
                    .message_params(exclusive.into(), num_shared)
                    .no_payload()
                    .write_to(stream)
                    .await?;
            }
            Message {
                message_type: MessageType::AsyncStartTLS,
                control_code,
                message_parameter,
                payload,
            } if self.protocol >= PROTOCOL_2_0 => {
                if payload.len() != 4 {
                    send_fatal!(peer=peer.to_string(), session_id=self.id;
                        stream, FatalErrorCode::PoorlyFormattedMessageHeader,
                        "Expected 4 bytes in AsyncStartTLS payload"
                    )
                }

                let control = RmtDeliveredControl(control_code);
                let message_id_sent = message_parameter;
                let message_id_read = NetworkEndian::read_u32(&payload);

                log::debug!(session_id=self.id, message_id_sent=message_id_sent, message_id_read=message_id_read; "Start async TLS");

                #[cfg(feature = "tls")]
                let control_code = if self.in_buf.is_empty() {
                    match stream.start_tls(acceptor).await {
                        Ok(_) => HISLIP_TLS_SUCCESS,
                        Err(_) => HISLIP_TLS_ERROR,
                    }
                } else {
                    HISLIP_TLS_BUSY
                };

                #[cfg(not(feature = "tls"))]
                let control_code = HISLIP_TLS_ERROR;

                MessageType::AsyncStartTLSResponse
                    .message_params(control_code, 0)
                    .no_payload()
                    .write_to(stream)
                    .await?;
            }
            Message {
                message_type: MessageType::AsyncEndTLS,
                ..
            } if self.protocol >= PROTOCOL_2_0 => {
                // Only supported >= 2.0

                #[cfg(feature = "tls")]
                let control_code = if self.config.encryption_mode {
                    HISLIP_TLS_ERROR
                } else if !self.in_buf.is_empty() {
                    HISLIP_TLS_BUSY
                } else {
                    match stream.end_tls().await {
                        Ok(_) => HISLIP_TLS_SUCCESS,
                        Err(_) => HISLIP_TLS_ERROR,
                    }
                };

                #[cfg(not(feature = "tls"))]
                let control_code = HISLIP_TLS_ERROR;

                MessageType::AsyncEndTLSResponse
                    .message_params(control_code, 0)
                    .no_payload()
                    .write_to(stream)
                    .await?;
            }
            _ => {
                send_nonfatal!(peer=peer.to_string(), session_id=self.id; stream,
                    NonFatalErrorCode::UnrecognizedMessageType,
                    "Unexpected message type in asynchronous channel",
                );
            }
        }
        Ok(())
    }

    /// Get the session's state.
    #[must_use]
    pub(crate) fn state(&self) -> SessionState {
        self.state
    }

    #[must_use]
    pub(crate) fn is_initialized(&self) -> bool {
        // !matches!(self.state, SessionState::Handshake) // Just looks weird
        match self.state {
            SessionState::Handshake => false,
            _ => false,
        }
    }

    /// Get the session's protocol.
    #[must_use]
    pub(crate) fn protocol(&self) -> Protocol {
        self.protocol
    }

    /// Set the session's state.
    pub(crate) fn set_state(&mut self, state: SessionState) {
        self.state = state;
    }
}

impl<DEV> Drop for Session<DEV>
where
    DEV: Device,
{
    fn drop(&mut self) {
        self.close()
    }
}
