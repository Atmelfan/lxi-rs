use std::cmp::min;
use std::collections::HashMap;
use std::io;
use std::str::from_utf8;
use std::sync::Weak;
use std::time::Duration;

use async_std::future;
use async_std::sync::Arc;
use async_std::{
    net::{TcpListener, TcpStream, ToSocketAddrs},
    task,
};
use byteorder::{ByteOrder, NetworkEndian};
use futures::StreamExt;
use lxi_device::lock::{LockHandle, Mutex, SharedLock, SpinMutex};
use lxi_device::Device;

use crate::common::errors::{Error, FatalErrorCode, NonFatalErrorCode};
use crate::common::messages::{
    AsyncInitializeResponseControl, AsyncInitializeResponseParameter, FeatureBitmap,
    InitializeParameter, InitializeResponseControl, InitializeResponseParameter, Message,
    MessageType, RmtDeliveredControl,
};
use crate::common::{Protocol, PROTOCOL_2_0, SUPPORTED_PROTOCOL};
use crate::server::session::{Session, SessionMode, SessionState};
use crate::server::stream::HislipStream;

#[cfg(feature = "tls")]
use async_tls::TlsAcceptor;

pub mod session;
mod stream;

#[derive(Debug, Copy, Clone)]
pub struct ServerConfig {
    pub vendor_id: u16,
    /// Maximum server message size
    pub max_message_size: u64,
    /// Prefer overlapped data
    pub prefer_overlap: bool,
    /// Mandatory encryption if true
    pub encryption_mode: bool,
    /// Require
    pub initial_encryption: bool,
    pub max_num_sessions: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            vendor_id: 0xBEEF,
            max_message_size: 1024,
            prefer_overlap: true,
            encryption_mode: false,
            initial_encryption: false,
            max_num_sessions: 64,
        }
    }
}

pub struct Server<DEV>
where
    DEV: Device,
{
    inner: Arc<Mutex<InnerServer<DEV>>>,
    shared_lock: Arc<SpinMutex<SharedLock>>,
    device: Arc<Mutex<DEV>>,
    config: ServerConfig,
}

impl<DEV> Server<DEV>
where
    DEV: Device + Send + 'static,
{
    pub fn new(shared_lock: Arc<SpinMutex<SharedLock>>, device: Arc<Mutex<DEV>>) -> Arc<Self> {
        let config = ServerConfig::default();
        Arc::new(Server {
            inner: InnerServer::new(config.max_num_sessions),
            config,
            shared_lock,
            device,
        })
    }

    /// Start accepting connections from addr
    ///
    pub async fn accept(
        self: Arc<Self>,
        addr: impl ToSocketAddrs,
        #[cfg(feature = "tls")] acceptor: Arc<TlsAcceptor>,
    ) -> Result<(), io::Error> {
        let listener = TcpListener::bind(addr).await?;
        let mut incoming = listener.incoming();
        while let Some(stream) = incoming.next().await {
            let stream = stream?;
            let peer = stream.peer_addr()?;
            #[cfg(feature = "tls")]
            let acceptor = acceptor.clone();

            let s = self.clone();
            task::spawn(async move {
                let res = s
                    .handle_connection(
                        stream,
                        #[cfg(feature = "tls")]
                        acceptor,
                    )
                    .await;
                log::debug!("{peer} disconnected: {res:?}")
            });
        }
        Ok(())
    }

    /// The connection handling function.
    async fn handle_connection(
        self: Arc<Self>,
        stream: TcpStream,
        #[cfg(feature = "tls")] acceptor: Arc<TlsAcceptor>,
    ) -> Result<(), io::Error> {
        let peer = stream.peer_addr()?;
        log::info!("{} connected", peer);

        let mut stream = HislipStream::Open(&stream);

        // Start reading packets from stream
        loop {
            match Message::read_from(&mut stream, self.config.max_message_size).await? {
                Ok(msg) => {
                    log::trace!("Received {:?}", msg);
                    // Handle messages
                    match msg {
                        Message {
                            message_type: MessageType::VendorSpecific(code),
                            ..
                        } => {
                            log::warn!(peer=format!("{}", peer);
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
                            message_type: MessageType::Initialize,
                            message_parameter,
                            payload,
                            ..
                        } => {
                            // Create new session
                            let client_parameters = InitializeParameter(message_parameter);

                            // TODO: Accept other than hislip0
                            if !payload.eq_ignore_ascii_case(b"hislip0") {
                                Message::from(Error::Fatal(
                                    FatalErrorCode::InvalidInitialization,
                                    b"Invalid sub adress",
                                ))
                                .write_to(&mut stream)
                                .await?;
                                break;
                            }
                            log::debug!(peer=format!("{}", peer);
                                "Sync initialize, version={}, vendor={}",
                                client_parameters.client_protocol(),
                                client_parameters.client_vendorid()
                            );

                            // Check if negotiated protocol is compatible with mandatory encryption
                            let lowest_protocol =
                                min(SUPPORTED_PROTOCOL, client_parameters.client_protocol());
                            if self.config.encryption_mode && lowest_protocol < PROTOCOL_2_0 {
                                log::error!(peer=format!("{}", peer);
                                    "Client does not support mandatory encryption"
                                );
                                Message::from(Error::Fatal(
                                    FatalErrorCode::SecureConnectionFailed,
                                    b"Secure connection failed",
                                ))
                                .write_to(&mut stream)
                                .await?;
                                break;
                            }

                            // Create new session
                            let (session_id, session) = {
                                let mut guard = self.inner.lock().await;
                                let handle =
                                    LockHandle::new(self.shared_lock.clone(), self.device.clone());
                                match guard.new_session(self.config, lowest_protocol, handle) {
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

                            // Negotiate encryption settings
                            let encryption_mode =
                                self.config.encryption_mode && lowest_protocol >= PROTOCOL_2_0;
                            let initial_encryption =
                                self.config.initial_encryption && lowest_protocol >= PROTOCOL_2_0;

                            let response_param =
                                InitializeResponseParameter::new(lowest_protocol, session_id);

                            let control = InitializeResponseControl::new(
                                self.config.prefer_overlap,
                                encryption_mode,
                                initial_encryption,
                            );

                            // Send response
                            log::debug!(peer=format!("{}", peer); "New session 0x{:04x}", session_id);
                            MessageType::InitializeResponse
                                .message_params(control.0, response_param.0)
                                .no_payload()
                                .write_to(&mut stream)
                                .await?;

                            break Session::handle_sync_session(
                                session,
                                &mut stream,
                                peer,
                                self.config,
                                #[cfg(feature = "tls")]
                                acceptor,
                            )
                            .await?;
                        }
                        Message {
                            message_type: MessageType::AsyncInitialize,
                            message_parameter,
                            ..
                        } => {
                            // Connect to existing session
                            let session_id = (message_parameter & 0x0000FFFF) as u16;
                            let session = {
                                let mut guard = self.inner.lock().await;
                                if let Some(s) = guard.get_session(session_id) {
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
                            let s = session.clone();
                            let mut session_guard = s.lock().await;

                            // Check if async channel has alreasy been initialized for this session
                            if session_guard.state() != SessionState::Handshake {
                                log::warn!(peer=format!("{}", peer);
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
                            log::debug!(peer=format!("{}", peer), session_id=session_id; "Async initialize");

                            // Support secure connection if "tls" feature is enabled and the agreed upon protocol is >= 2.0.
                            let secure_connection =
                                cfg!(feature = "tls") && session_guard.protocol() >= PROTOCOL_2_0;

                            MessageType::AsyncInitializeResponse
                                .message_params(
                                    AsyncInitializeResponseControl::new(secure_connection).0,
                                    AsyncInitializeResponseParameter::new(self.config.vendor_id).0,
                                )
                                .no_payload()
                                .write_to(&mut stream)
                                .await?;

                            // Require encryption transaction if encryption is mandatory or initially required
                            if cfg!(feature = "tls")
                                && (self.config.initial_encryption || self.config.encryption_mode)
                            {
                                session_guard.set_state(SessionState::EncryptionStart)
                            } else {
                                session_guard.set_state(SessionState::Normal)
                            }

                            drop(session_guard);

                            break Session::handle_async_session(
                                session,
                                &mut stream,
                                peer,
                                self.config,
                                #[cfg(feature = "tls")]
                                acceptor,
                            )
                            .await?;
                        }
                        msg => {
                            log::warn!(peer=format!("{}", peer); "Unexpected message {:?} during initialization", msg);
                            Message::from(Error::Fatal(
                                FatalErrorCode::InvalidInitialization,
                                b"Unexpected message during initialization",
                            ))
                            .write_to(&mut stream)
                            .await?;
                        }
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
        log::info!("{} disconnected", peer);
        Ok(())
    }
}

struct InnerServer<DEV>
where
    DEV: Device,
{
    session_id: u16,
    sessions: HashMap<u16, Weak<Mutex<Session<DEV>>>>,
    max_num_sessions: usize,
}

impl<DEV> InnerServer<DEV>
where
    DEV: Device,
{
    fn new(max_num_sessions: usize) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(InnerServer {
            session_id: 0,
            sessions: Default::default(),
            max_num_sessions,
        }))
    }

    /// Get next available session id
    fn new_session_id(&mut self) -> Result<u16, Error> {
        let origin = self.session_id;
        self.session_id = self.session_id.wrapping_add(2);

        // Check if session id already exists (wrapped around)
        while self.sessions.contains_key(&self.session_id) {
            self.session_id = self.session_id.wrapping_add(2);
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
        config: ServerConfig,
        protocol: Protocol,
        handle: LockHandle<DEV>,
    ) -> Result<(u16, Arc<Mutex<Session<DEV>>>), Error> {
        self.gc_sessions();
        if self.sessions.len() >= self.max_num_sessions {
            return Err(Error::Fatal(
                FatalErrorCode::MaximumClientsExceeded,
                b"Out of session ids",
            ));
        }

        let session_id = self.new_session_id()?;
        let session = Arc::new(Mutex::new(Session::new(
            config, session_id, protocol, handle,
        )));
        // Store a weak pointer so that the session gets dropped when both sync and async channels are dropped
        self.sessions.insert(session_id, Arc::downgrade(&session));
        Ok((session_id, session))
    }

    /// Get a session
    /// Note: Returns a strong reference which will keep any locks assosciated with session active until dropped
    fn get_session(&mut self, session_id: u16) -> Option<Arc<Mutex<Session<DEV>>>> {
        let session = self.sessions.get(&session_id)?;
        session.upgrade()
    }

    /// Remove any stale session id
    fn gc_sessions(&mut self) {
        self.sessions.retain(|_, v| v.strong_count() != 0)
    }
}
