use std::cmp::min;
use std::collections::HashMap;
use std::io;
use std::str::from_utf8;
use std::time::Duration;

use async_std::future;
use async_std::sync::Arc;
use async_std::{
    net::{TcpListener, TcpStream, ToSocketAddrs},
    task,
};
use byteorder::{ByteOrder, NetworkEndian};
use futures::lock::Mutex;
use futures::StreamExt;
use lxi_device::lock::{LockHandle, SharedLock, SharedLockError, SharedLockMode, SpinMutex};
use lxi_device::Device;

use crate::common::errors::{Error, FatalErrorCode, NonFatalErrorCode};
use crate::common::messages::{
    AsyncInitializeResponseControl, AsyncInitializeResponseParameter, FeatureBitmap,
    InitializeParameter, InitializeResponseControl, InitializeResponseParameter, Message,
    MessageType, RmtDeliveredControl,
};
use crate::common::Protocol;
use crate::server::session::{Session, SessionMode};
use crate::PROTOCOL_2_0;

pub mod session;
mod stream;

#[derive(Debug, Copy, Clone)]
pub struct ServerConfig {
    pub vendor_id: u16,
    /// Maximum server message size
    pub max_message_size: u64,
    pub preferred_mode: SessionMode,
    pub encryption_mandatory: bool,
    pub initial_encryption: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            vendor_id: 0xBEEF,
            max_message_size: 1024,
            preferred_mode: SessionMode::Synchronized,
            encryption_mandatory: false,
            initial_encryption: false,
        }
    }
}

pub struct Server<DEV> {
    inner: Arc<Mutex<InnerServer<DEV>>>,
    shared_lock: Arc<SpinMutex<SharedLock>>,
    device: Arc<Mutex<DEV>>,
    config: ServerConfig,
}

impl<DEV> Server<DEV>
where
    DEV: Device + Send + 'static,
{
    pub fn new(
        _vendor_id: u16,
        shared_lock: Arc<SpinMutex<SharedLock>>,
        device: Arc<Mutex<DEV>>,
    ) -> Arc<Self> {
        Arc::new(Server {
            inner: InnerServer::new(),
            config: ServerConfig::default(),
            shared_lock,
            device,
        })
    }

    /// Start accepting connections from addr
    ///
    pub async fn accept(self: Arc<Self>, addr: impl ToSocketAddrs) -> Result<(), io::Error> {
        let listener = TcpListener::bind(addr).await?;
        let mut incoming = listener.incoming();
        while let Some(stream) = incoming.next().await {
            let stream = stream?;
            let peer = stream.peer_addr()?;

            let s = self.clone();
            task::spawn(async move {
                let res = s.handle_connection(stream).await;
                if let Err(err) = res {
                    log::error!("{peer} disconnected: {err}")
                } else {
                    log::info!("{peer} disconnected")
                }
            });
        }
        Ok(())
    }

    async fn handle_async_message(
        self: Arc<Self>,
        session: &mut Session<DEV>,
        msg: Message,
        stream: &mut TcpStream,
    ) -> Result<(), io::Error> {
        match msg.message_type() {
            MessageType::AsyncLock => {
                if msg.control_code() == 0 {
                    // Release
                    let message_id = msg.message_parameter();
                    log::debug!(session_id = session.id, message_id = message_id; "Release async lock");
                    let control = match session.handle.try_release() {
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
                    let timeout = msg.message_parameter();
                    let lockstr = if let Ok(s) = std::str::from_utf8(msg.payload()) {
                        s.trim_end_matches('\0')
                    } else {
                        log::error!(session_id = session.id, timeout=timeout; "Lockstr is not valid UTF8");
                        Message::from(Error::Fatal(
                            FatalErrorCode::UnidentifiedError,
                            b"Lockstr is not valid UTF8",
                        ))
                        .write_to(stream)
                        .await?;
                        return Err(io::ErrorKind::Other.into());
                    };
                    log::debug!(session_id = session.id, timeout=timeout; "Async lock: '{}'", lockstr);

                    // Try to acquire lock
                    let res = if timeout == 0 {
                        // Try to lock immediately
                        if lockstr.is_empty() {
                            session.handle.try_acquire_exclusive()
                        } else {
                            session.handle.try_acquire_shared(lockstr)
                        }
                    } else {
                        // Try to lock until timed out
                        if lockstr.is_empty() {
                            future::timeout(
                                Duration::from_millis(timeout as u64),
                                session.handle.async_acquire_exclusive(),
                            )
                            .await
                            .map_err(|_| SharedLockError::Timeout)
                            .and_then(|res| res)
                        } else {
                            future::timeout(
                                Duration::from_millis(timeout as u64),
                                session.handle.async_acquire_shared(lockstr),
                            )
                            .await
                            .map_err(|_| SharedLockError::Timeout)
                            .and_then(|res| res)
                        }
                    };

                    log::debug!(session_id = session.id; "Async lock: {:?}", res);

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
            MessageType::AsyncRemoteLocalControl => todo!(),
            MessageType::AsyncInterrupted => todo!(),
            MessageType::AsyncMaximumMessageSize => {
                let size = NetworkEndian::read_u64(msg.payload().as_slice());
                session.max_message_size = size;
                log::debug!("Session {}, Max client message size = {}", session.id, size);
                log::debug!(session_id = session.id; "Max client message size = {}", size);

                let mut buf = [0u8; 8];

                NetworkEndian::write_u64(&mut buf, self.config.max_message_size as u64);
                MessageType::AsyncMaximumMessageSizeResponse
                    .message_params(0, 0)
                    .with_payload(buf.to_vec())
                    .write_to(stream)
                    .await?;
            }
            MessageType::AsyncDeviceClear => {
                log::debug!(session_id = session.id; "Device clear");
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
            MessageType::AsyncServiceRequest => todo!(),
            MessageType::AsyncStatusQuery => todo!(),
            MessageType::AsyncLockInfo => {
                let shared_lock = self.shared_lock.lock();
                log::debug!(session_id = session.id; "Lock info, exclusive={}, shared={}",
                    shared_lock.exclusive_lock(),
                    shared_lock.num_shared_locks()
                );

                MessageType::AsyncLockInfoResponse
                    .message_params(
                        shared_lock.exclusive_lock().into(),
                        shared_lock.num_shared_locks(),
                    )
                    .no_payload()
                    .write_to(stream)
                    .await?;
            }
            MessageType::AsyncStartTLS => todo!(),
            MessageType::AsyncEndTLS => todo!(),
            _ => {
                log::debug!(session_id = session.id; "Unexpected message type in asynchronous channel");
                Message::from(Error::Fatal(
                    FatalErrorCode::InvalidInitialization,
                    b"Unexpected messagein asynchronous channel",
                ))
                .write_to(stream)
                .await?;
                return Err(io::ErrorKind::Other.into());
            }
        }
        Ok(())
    }

    /// The connection handling function.
    async fn handle_connection(self: Arc<Self>, mut stream: TcpStream) -> Result<(), io::Error> {
        let peer_addr = stream.peer_addr()?;
        log::info!("{} connected", peer_addr);

        let mut connection_state = ConnectionState::Handshake;

        // Start reading packets from stream
        loop {
            match Message::read_from(&mut stream, self.config.max_message_size).await? {
                Ok(msg) => {
                    log::trace!("Received {:?}", msg);
                    // Handle messages
                    match msg.message_type() {
                        MessageType::VendorSpecific(code) => {
                            log::warn!(
                                "Unrecognized Vendor Defined Message ({}) during init",
                                code
                            );
                            Message::from(Error::NonFatal(
                                NonFatalErrorCode::UnrecognizedVendorDefinedMessage,
                                b"Unrecognized Vendor Defined Message",
                            ))
                            .write_to(&mut stream)
                            .await?;
                        }
                        MessageType::FatalError => {
                            log::error!(
                                "Client fatal error: {}",
                                from_utf8(msg.payload()).unwrap_or("<invalid utf8>")
                            );
                            //break; // Let client close connection
                        }
                        MessageType::Error => {
                            log::warn!(
                                "Client error: {}",
                                from_utf8(msg.payload()).unwrap_or("<invalid utf8>")
                            );
                        }
                        others => match &connection_state {
                            // Currently doing handshake
                            ConnectionState::Handshake => match others {
                                MessageType::Initialize => {
                                    if !msg.payload().is_ascii() {
                                        Message::from(Error::Fatal(
                                            FatalErrorCode::InvalidInitialization,
                                            b"Invalid sub-adress",
                                        ))
                                        .write_to(&mut stream)
                                        .await?;
                                        break;
                                    }

                                    // Create new session
                                    let client_parameters =
                                        InitializeParameter(msg.message_parameter());

                                    // TODO: Accept other than hislip0
                                    if !msg.payload().eq_ignore_ascii_case(b"hislip0") {
                                        Message::from(Error::Fatal(
                                            FatalErrorCode::InvalidInitialization,
                                            b"Invalid sub adress",
                                        ))
                                        .write_to(&mut stream)
                                        .await?;
                                        break;
                                    }
                                    log::debug!(
                                        "Sync initialize, version={}, vendor={}",
                                        client_parameters.client_protocol(),
                                        client_parameters.client_vendorid()
                                    );

                                    let lowest_protocol =
                                        min(PROTOCOL_2_0, client_parameters.client_protocol());

                                    // Create new session
                                    let (session_id, session) = {
                                        let mut guard = self.inner.lock().await;
                                        let handle = LockHandle::new(
                                            self.shared_lock.clone(),
                                            self.device.clone(),
                                        );
                                        match guard.new_session(lowest_protocol, handle) {
                                            Ok(s) => s,
                                            Err(_err) => {
                                                Message::from(Error::Fatal(
                                                    FatalErrorCode::InvalidInitialization,
                                                    b"Already initialized",
                                                ))
                                                .write_to(&mut stream)
                                                .await?;
                                                break;
                                            }
                                        }
                                    };
                                    log::debug!("New session 0x{:04x}", session_id);

                                    // Send response
                                    let response_param = InitializeResponseParameter::new(
                                        lowest_protocol,
                                        session_id,
                                    );
                                    let control = InitializeResponseControl::new(
                                        self.config.preferred_mode == SessionMode::Overlapped,
                                        self.config.encryption_mandatory,
                                        self.config.initial_encryption,
                                    );

                                    // Connection is a synchronous channel
                                    connection_state = ConnectionState::Synchronous(session);

                                    MessageType::InitializeResponse
                                        .message_params(control.0, response_param.0)
                                        .no_payload()
                                        .write_to(&mut stream)
                                        .await?;
                                }
                                MessageType::AsyncInitialize => {
                                    // Connect to existing session
                                    let session_id = (msg.message_parameter() & 0x0000FFFF) as u16;
                                    let session = {
                                        let guard = self.inner.lock().await;
                                        if let Some(s) = guard.sessions.get(&session_id).cloned() {
                                            s
                                        } else {
                                            Message::from(Error::Fatal(
                                                FatalErrorCode::InvalidInitialization,
                                                b"Invalid session id",
                                            ))
                                            .write_to(&mut stream)
                                            .await?;
                                            break;
                                        }
                                    };
                                    let mut session_guard = session.lock().await;

                                    if session_guard.async_connected {
                                        log::warn!(
                                            "Async session 0x{:04x} already initialized",
                                            session_id
                                        );
                                        Message::from(Error::Fatal(
                                            FatalErrorCode::InvalidInitialization,
                                            b"Async already initialized",
                                        ))
                                        .write_to(&mut stream)
                                        .await?;
                                        break;
                                    }

                                    log::debug!("AsyncInitialize session=0x{:04x}", session_id);

                                    let parameter = AsyncInitializeResponseParameter::new(
                                        self.config.vendor_id,
                                    );
                                    let control = AsyncInitializeResponseControl::new(
                                        self.config.initial_encryption,
                                    );

                                    MessageType::AsyncInitializeResponse
                                        .message_params(control.0, parameter.0)
                                        .no_payload()
                                        .write_to(&mut stream)
                                        .await?;

                                    session_guard.async_connected = true;
                                    connection_state =
                                        ConnectionState::Asynchronous(session.clone());
                                }
                                _ => {
                                    log::error!("Unexpected message type during handshake");
                                    Message::from(Error::Fatal(
                                        FatalErrorCode::InvalidInitialization,
                                        b"Unexpected message",
                                    ))
                                    .write_to(&mut stream)
                                    .await?;
                                    break;
                                }
                            },

                            ConnectionState::Asynchronous(s) => {
                                let mut session = s.lock().await;
                                if let Err(err) = self
                                    .clone()
                                    .handle_async_message(&mut session, msg, &mut stream)
                                    .await
                                {
                                    // Close session
                                    let mut inner = self.inner.lock().await;
                                    session.close();
                                    inner.sessions.remove(&session.id);
                                    return Err(err);
                                }
                            }
                            ConnectionState::Synchronous(s) => {
                                let session = s.lock().await;
                                if !session.async_connected {
                                    Message::from(Error::Fatal(
                                        FatalErrorCode::AttemptUseWithoutBothChannels,
                                        b"Attempted to use without both channels",
                                    ))
                                    .write_to(&mut stream)
                                    .await?;
                                    break;
                                }

                                match others {
                                    MessageType::Data | MessageType::DataEnd => {
                                        let control = RmtDeliveredControl(msg.control_code());
                                        let messageid = msg.message_parameter();
                                        let end = matches!(others, MessageType::DataEnd);
                                        log::debug!(
                                            "Session {}, Data, RMT-delivered={}, messageID={}, end={}, size={}",
                                            session.id,
                                            control.rmt_delivered(),
                                            messageid,
                                            end,
                                            msg.payload().len()
                                        );
                                    }
                                    MessageType::DeviceClearComplete => todo!(),
                                    MessageType::Trigger => todo!(),
                                    MessageType::Interrupted => todo!(),
                                    MessageType::GetDescriptors => todo!(),
                                    MessageType::StartTLS => todo!(),
                                    MessageType::EndTLS => todo!(),
                                    MessageType::GetSaslMechanismList => todo!(),
                                    MessageType::GetSaslMechanismListResponse => todo!(),
                                    MessageType::AuthenticationStart => todo!(),
                                    MessageType::AuthenticationExchange => todo!(),
                                    MessageType::AuthenticationResult => todo!(),
                                    _ => {
                                        log::error!(
                                            "Unexpected message type in synchronous channel"
                                        );
                                        Message::from(Error::Fatal(
                                            FatalErrorCode::InvalidInitialization,
                                            b"Unexpected message in synchronous channel",
                                        ))
                                        .write_to(&mut stream)
                                        .await?;
                                        break;
                                    }
                                }
                            }
                        },
                    }
                }
                Err(err) => {
                    Message::from(err).write_to(&mut stream).await?;
                    if err.is_fatal() {
                        break;
                    }
                }
            }
        }

        // Close connection
        drop(stream);
        log::info!("{} disconnected", peer_addr);
        Ok(())
    }
}

enum ConnectionState<DEV> {
    Handshake,
    Synchronous(Arc<Mutex<Session<DEV>>>),
    Asynchronous(Arc<Mutex<Session<DEV>>>),
}
struct InnerServer<DEV> {
    session_id: u16,
    sessions: HashMap<u16, Arc<Mutex<Session<DEV>>>>,
}

impl<DEV> InnerServer<DEV> {
    fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(InnerServer {
            session_id: 0,
            sessions: Default::default(),
        }))
    }

    /// Get next available session id
    fn new_session_id(&mut self) -> Result<u16, Error> {
        let origin = self.session_id;
        self.session_id += 2;
        // Check if key already exists (wrapped around)
        while self.sessions.contains_key(&self.session_id) {
            self.session_id += 2;
            // Back at beginning, no more ids...
            if self.session_id == origin {
                return Err(Error::Fatal(
                    FatalErrorCode::MaximumClientsExceeded,
                    b"Out of session ids",
                )
                .into());
            }
        }

        Ok(self.session_id)
    }

    fn new_session(
        &mut self,
        protocol: Protocol,
        handle: LockHandle<DEV>,
    ) -> Result<(u16, Arc<Mutex<Session<DEV>>>), Error> {
        let session_id = self.new_session_id()?;
        let session = Arc::new(Mutex::new(Session::new(session_id, protocol, handle)));
        self.sessions.insert(session_id, session.clone());
        Ok((session_id, session))
    }
}
