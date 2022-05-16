use std::io;
use std::net::SocketAddr;
use std::str::from_utf8;
use std::time::Duration;

use async_std::channel::{Receiver, Sender};
use async_std::future;
use async_std::net::TcpStream;
use async_std::sync::Arc;
use byteorder::{ByteOrder, NetworkEndian};
use futures::channel::mpsc;
use futures::lock::Mutex;
use futures::{StreamExt, select, FutureExt};
use lxi_device::lock::{LockHandle, SharedLockError, SharedLockMode};
use lxi_device::Device;

use crate::common::errors::{Error, FatalErrorCode, NonFatalErrorCode};
use crate::common::messages::{FeatureBitmap, Message, MessageType, RmtDeliveredControl};
use crate::common::Protocol;
use crate::PROTOCOL_2_0;

use super::stream::{HislipStream, HISLIP_TLS_BUSY, HISLIP_TLS_ERROR, HISLIP_TLS_SUCCESS};
use super::ServerConfig;

#[cfg(feature = "tls")]
use async_tls::TlsAcceptor;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SessionMode {
    Synchronized,
    Overlapped,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub(crate) enum SessionState {
    Normal,
    Clear,
    EncryptionStart,
    AuthenticationStart,
}

pub(crate) enum SyncEvent {

}

pub(crate) struct Session<DEV>
where
    DEV: Device,
{
    config: ServerConfig,
    /// Negotiated rpc
    pub(crate) protocol: Protocol,
    /// Negotiated session mode
    pub(crate) mode: SessionMode,
    /// Session ID
    pub(crate) id: u16,
    /// Client max message size
    pub(crate) max_message_size: u64,

    // Internal statekeeping between async and sync channel
    pub(crate) async_connected: bool,
    pub(crate) async_encrypted: bool,

    pub(crate) handle: LockHandle<DEV>,

    // Input/Output buffer
    pub(crate) in_buf: Vec<u8>,
    pub(crate) out_buf: Vec<u8>,
    message_id: u32,

    state: SessionState,

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
            async_connected: false,
            async_encrypted: false,
            handle,
            in_buf: Vec::new(),
            out_buf: Vec::new(),
            enable_remote: true,
            message_id: 0xffff_ff00,
            state: SessionState::Normal
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
        #[cfg(feature = "tls")] acceptor: Arc<TlsAcceptor>
    ) -> Result<(), io::Error> {
        loop {
            match Message::read_from(stream, config.max_message_size).await? {
                // Valid message
                Ok(msg) => {
                    let mut session = session.lock().await;
                    session.handle_sync_message(msg, stream, peer, #[cfg(feature = "tls")] acceptor.clone()).await?;
                }
                // Invalid message
                Err(err) => {
                    Message::from(err).write_to(stream).await?;
                    if err.is_fatal() {
                        break Ok(());
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
        #[cfg(feature = "tls")] acceptor: Arc<TlsAcceptor>
    ) -> Result<(), io::Error> {
        match msg {
            Message {
                message_type: MessageType::VendorSpecific(code),
                ..
            } => {
                log::warn!(peer=format!("{}", peer);
                    "Unrecognized Vendor Defined Message ({})",
                    code
                );
                Message::from(Error::NonFatal(
                    NonFatalErrorCode::UnrecognizedVendorDefinedMessage,
                    b"Unrecognized Vendor Defined Message",
                ))
                .write_to(stream)
                .await?;
            }
            Message {
                message_type: MessageType::FatalError,
                control_code,
                payload,
                ..
            } => {
                log::error!(peer=format!("{}", peer);
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
                log::warn!(peer=format!("{}", peer);
                    "Client error {:?}: {}", NonFatalErrorCode::from_error_code(control_code),
                    from_utf8(&payload).unwrap_or("<invalid utf8>")
                );
            }
            Message {
                message_type: MessageType::Data,
                message_parameter: message_id,
                payload,
                ..
            } => {
                if self.state == SessionState::Clear {
                    // Ignore data when clearing
                    return Ok(())
                }

                let mut dev = self.handle.async_lock().await.unwrap();

                //
                if self.enable_remote {
                    dev.set_remote(true);
                }

                //
                self.in_buf.extend_from_slice(&payload);
                log::trace!(peer=format!("{}", peer), message_id=message_id; "Data {:?}", payload);
            }
            Message {
                message_type: MessageType::DataEnd,
                message_parameter: message_id,
                payload,
                ..
            } => {
                if self.state == SessionState::Clear {
                    // Ignore data when clearing
                    return Ok(())
                }

                let mut dev = self.handle.async_lock().await.unwrap();

                //
                if self.enable_remote {
                    dev.set_remote(true);
                }

                //
                self.in_buf.extend_from_slice(&payload);
                log::trace!(peer=format!("{}", peer), message_id=message_id; "DataEnd {:?}", payload);
                let out = dev.execute(&self.in_buf);
                self.in_buf.clear();

                // Send back any output data
                if !out.is_empty() {
                    let mut iter = out.chunks_exact(self.max_message_size as usize);
                    while let Some(chunk) = iter.next() {
                        MessageType::Data
                            .message_params(0, self.message_id)
                            .with_payload(chunk.to_vec())
                            .write_to(stream)
                            .await?;
                        self.message_id = self.message_id.wrapping_add(2);
                    }
                    MessageType::DataEnd
                        .message_params(0, self.message_id)
                        .with_payload(iter.remainder().to_vec())
                        .write_to(stream)
                        .await?;
                    self.message_id = self.message_id.wrapping_add(2);
                }
            }
            Message {
                message_type: MessageType::Trigger,
                message_parameter: message_id,
                control_code,
                ..
            } => {
                if self.state == SessionState::Clear {
                    // Ignore data when clearing
                    return Ok(())
                }

                let control = RmtDeliveredControl(control_code);
                log::debug!(session_id = self.id, message_id=message_id; "Trigger, {}", control);


                let mut dev = self.handle.async_lock().await.unwrap();

                if self.enable_remote {
                    dev.set_remote(true);
                }
                let _ = dev.trigger();
            },
            Message {
                message_type: MessageType::DeviceClearComplete,
                control_code,
                ..
            } => {
                if self.state != SessionState::Clear {
                    // No clear has been commanded
                    Message::from(Error::NonFatal(
                        NonFatalErrorCode::UnidentifiedError,
                        b"Unexpected device clear complete",
                    ))
                    .write_to(stream)
                    .await?;
                } else {
                    let feature_request = FeatureBitmap(control_code);
                    log::debug!(session_id = self.id; "Device clear complete, {}", feature_request);
    
                    self.state = SessionState::Normal;
                    // Renegotiate
                    self.mode = if feature_request.overlapped() {
                        SessionMode::Overlapped
                    } else {
                        SessionMode::Synchronized
                    };
                    let feature_setting = FeatureBitmap::new(feature_request.overlapped(), self.config.encryption_mandatory, self.config.initial_encryption);
                    MessageType::DeviceClearAcknowledge
                            .message_params(feature_setting.0, self.message_id)
                            .no_payload()
                            .write_to(stream)
                            .await?;
                }
            }
            msg => {
                log::debug!(session_id = self.id; "Unexpected message type in synchronous channel: {:?}", msg);
                Message::from(Error::Fatal(
                    FatalErrorCode::UnidentifiedError,
                    b"Unexpected message in synchronous channel",
                ))
                .write_to(stream)
                .await?;
                return Err(io::ErrorKind::Other.into());
            }
        }
        Ok(())
    }

    pub(crate) async fn handle_async_session(
        session: Arc<Mutex<Self>>,
        stream: &mut HislipStream<'_>,
        peer: SocketAddr,
        config: ServerConfig,
        #[cfg(feature = "tls")] acceptor: Arc<TlsAcceptor>
    ) -> Result<(), io::Error> {
        loop {
            match Message::read_from(stream, config.max_message_size).await? {
                Ok(msg) => {
                    let mut session = session.lock().await;
                    session.handle_async_message(msg, stream, peer, #[cfg(feature = "tls")] acceptor.clone()).await?;
                }
                Err(err) => {
                    Message::from(err).write_to(stream).await?;
                    if err.is_fatal() {
                        break Ok(());
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
        #[cfg(feature = "tls")] acceptor: Arc<TlsAcceptor>
    ) -> Result<(), io::Error> {
        match msg {
            Message {
                message_type: MessageType::VendorSpecific(code),
                ..
            } => {
                log::warn!(peer=format!("{}", peer);
                    "Unrecognized Vendor Defined Message ({})",
                    code
                );
                Message::from(Error::NonFatal(
                    NonFatalErrorCode::UnrecognizedVendorDefinedMessage,
                    b"Unrecognized Vendor Defined Message",
                ))
                .write_to(stream)
                .await?;
            }
            Message {
                message_type: MessageType::FatalError,
                control_code,
                payload,
                ..
            } => {
                log::error!(peer=format!("{}", peer);
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
                log::warn!(peer=format!("{}", peer);
                    "Client error {:?}: {}", NonFatalErrorCode::from_error_code(control_code),
                    from_utf8(&payload).unwrap_or("<invalid utf8>")
                );
            }
            Message {
                message_type: MessageType::AsyncLock,
                message_parameter,
                control_code,
                payload: lockstr
            } => {
                if control_code == 0 {
                    // Release
                    let message_id = message_parameter;
                    log::debug!(session_id = self.id, message_id = message_id; "Release async lock");
                    let control = match self.handle.try_release() {
                        Ok(SharedLockMode::Exclusive) => 1,
                        Ok(SharedLockMode::Shared) => 2,
                        Err(_) => 3,
                    };
                    MessageType::AsyncLockResponse
                        .message_params(control, 0)
                        .no_payload()
                        .write_to(stream)
                        .await?;
                } else {
                    // Lock
                    let timeout = message_parameter;
                    if !lockstr.is_ascii() {
                        log::error!(session_id = self.id, timeout=timeout; "Lockstr is not valid ASCII");
                        Message::from(Error::Fatal(
                            FatalErrorCode::UnidentifiedError,
                            b"Lockstr is not valid ASCII",
                        ))
                        .write_to(stream)
                        .await?;
                        return Err(io::ErrorKind::Other.into());
                    };
                    log::debug!(session_id = self.id, timeout=timeout; "Async lock: {:?}", lockstr);

                    // Try to acquire lock
                    let res = if timeout == 0 {
                        // Try to lock immediately
                        self.handle.try_acquire(&lockstr[..])
                    } else {
                        // Try to lock until timed out
                        future::timeout(
                            Duration::from_millis(timeout as u64),
                            self.handle.async_acquire(&lockstr[..]),
                        )
                        .await
                        .map_err(|_| SharedLockError::Timeout)
                        .and_then(|res| res)
                    };

                    //log::debug!(session_id = self.id; "Async lock: {:?}", res);

                    let control = match res {
                        Ok(_) => 1,
                        Err(SharedLockError::AlreadyLocked) => 3,
                        Err(_) => 0,
                    };

                    MessageType::AsyncLockResponse
                        .message_params(control, 0)
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
                log::debug!(session_id=self.id, message_id=message_id; "Remote/local request = {}", request);
                let mut dev = self.handle.async_lock().await.unwrap();
                let res = match request {
                    0 | 2 => {
                        dev.set_local_lockout(false);
                        self.enable_remote = false;
                        dev.set_remote(false)
                    }
                    1 => {
                        self.enable_remote = true;
                        Ok(())
                    }
                    3 => {
                        self.enable_remote = true;
                        dev.set_remote(false)
                    }
                    4 => {
                        dev.set_local_lockout(true);
                        dev.set_remote(true)
                    }
                    5 => {
                        dev.set_local_lockout(true);
                        self.enable_remote = true;
                        dev.set_remote(true)
                    }
                    6 => {
                        self.enable_remote = false;
                        Ok(())
                    }
                    _ => {
                        Message::from(Error::NonFatal(
                            NonFatalErrorCode::UnrecognizedControlCode,
                            b"Unexpected message in asynchronous channel",
                        ))
                        .write_to(stream)
                        .await?;
                        return Ok(());
                    }
                };
                match res {
                    Ok(_) => {
                        MessageType::AsyncRemoteLocalResponse
                            .message_params(0, 0)
                            .no_payload()
                            .write_to(stream)
                            .await?
                    }
                    Err(err) => {
                        log::error!(session_id=self.id, message_id=message_id; "Failed to remote/local: {:?}", err);
                        Message::from(Error::NonFatal(
                            NonFatalErrorCode::UnidentifiedError,
                            b"Internal error",
                        ))
                        .write_to(stream)
                        .await?;
                        return Ok(());
                    }
                }
            }
            Message {
                message_type: MessageType::AsyncMaximumMessageSize,
                payload,
                ..
            } => {
                if payload.len() < 8 {
                    return Err(io::ErrorKind::UnexpectedEof.into());
                }
                let size = NetworkEndian::read_u64(payload.as_slice());
                self.max_message_size = size;
                log::debug!(session_id=self.id; "Max client message size = {}", size);

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
                self.message_id = 0xffff_ff00;

                let features = FeatureBitmap::new(
                    self.config.preferred_mode == SessionMode::Overlapped,
                    self.config.encryption_mandatory,
                    self.config.initial_encryption,
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
                message_parameter,
                ..
            } => {
                // Only supported >= 2.0
                if self.protocol < PROTOCOL_2_0 {
                    log::debug!(session_id = self.id; "Unexpected message type in asynchronous channel");
                    Message::from(Error::NonFatal(
                        NonFatalErrorCode::UnrecognizedMessageType,
                        b"Unexpected message in asynchronous channel",
                    ))
                    .write_to(stream)
                    .await?;
                    return Ok(());
                }

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
            } => {
                // Only supported >= 2.0
                if self.protocol < PROTOCOL_2_0 {
                    log::debug!(session_id = self.id; "Unexpected message type in asynchronous channel");
                    Message::from(Error::NonFatal(
                        NonFatalErrorCode::UnrecognizedMessageType,
                        b"Unexpected message in asynchronous channel",
                    ))
                    .write_to(stream)
                    .await?;
                    return Ok(());
                }

                #[cfg(feature = "tls")]
                let control_code = if self.config.encryption_mandatory {
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
                log::debug!(session_id = self.id; "Unexpected message type in asynchronous channel");
                Message::from(Error::Fatal(
                    FatalErrorCode::UnidentifiedError,
                    b"Unexpected message in asynchronous channel",
                ))
                .write_to(stream)
                .await?;
                return Err(io::ErrorKind::Other.into());
            }
        }
        Ok(())
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
