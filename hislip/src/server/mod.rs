use std::cmp::min;
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::str::from_utf8;
use std::sync::Weak;

use async_std::sync::Arc;
use async_std::{
    net::{TcpListener, TcpStream, ToSocketAddrs},
    task,
};

use futures::StreamExt;
use lxi_device::lock::{LockHandle, Mutex, SharedLock, SpinMutex};
use lxi_device::Device;

use crate::common::errors::{Error, FatalErrorCode, NonFatalErrorCode};
use crate::common::messages::{prelude::*, send_fatal, send_nonfatal};
use crate::common::{Protocol, PROTOCOL_2_0, SUPPORTED_PROTOCOL};
use crate::server::session::{AsyncSession, Session, SessionState, SyncSession};
use crate::server::stream::HislipStream;

#[cfg(feature = "tls")]
use async_tls::TlsAcceptor;
#[cfg(feature = "tls")]
use sasl::{secret, server::Validator};

pub mod auth;
use auth::Auth;

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

pub struct Server<DEV, A>
where
    DEV: Device,
    A: Auth,
{
    inner: Arc<Mutex<InnerServer<DEV>>>,
    shared_lock: Arc<SpinMutex<SharedLock>>,
    device: Arc<Mutex<DEV>>,
    config: ServerConfig,
    authenticator: Arc<Mutex<A>>,
}

impl<DEV, A> Server<DEV, A>
where
    DEV: Device + Send + 'static,
    A: Auth + Send + 'static,
{
    pub fn new(
        shared_lock: Arc<SpinMutex<SharedLock>>,
        device: Arc<Mutex<DEV>>,
        authenticator: Arc<Mutex<A>>,
    ) -> Arc<Self> {
        let config = ServerConfig::default();
        Arc::new(Server {
            inner: InnerServer::new(config.max_num_sessions),
            config,
            shared_lock,
            device,
            authenticator,
        })
    }

    /// Start accepting connections from addr
    ///
    pub async fn accept(
        self: Arc<Self>,
        addr: impl ToSocketAddrs,
        #[cfg(feature = "tls")] acceptor: TlsAcceptor,
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
                    .handle_session(
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

    async fn handle_session(
        self: Arc<Self>,
        stream: TcpStream,
        #[cfg(feature = "tls")] acceptor: TlsAcceptor,
    ) -> Result<(), io::Error> {
        let peer = stream.peer_addr()?;
        log::info!("{} connected", peer);

        let mut stream = HislipStream::Open(&stream);

        loop {
            match Message::read_from(&mut stream, self.config.max_message_size).await? {
                Ok(msg) => {
                    log::trace!("Received {:?}", msg);
                    match msg {
                        Message {
                            message_type: MessageType::VendorSpecific(code),
                            ..
                        } => {
                            send_nonfatal!(peer=format!("{}", peer);
                                &mut stream, NonFatalErrorCode::UnrecognizedVendorDefinedMessage,
                                "Unrecognized Vendor Defined Message ({}) during init",
                                code
                            )
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
                                let s = std::str::from_utf8(&payload).unwrap_or("<invalid utf8>");
                                send_fatal!(peer=format!("{}", peer); &mut stream, FatalErrorCode::InvalidInitialization, "Invalid sub adress: {}", s)
                            }

                            log::debug!(peer=format!("{}", peer);
                                "Sync initialize, version={}, vendor={}",
                                client_parameters.client_protocol(),
                                client_parameters.client_vendorid()
                            );

                            // Check if negotiated protocol is compatible with mandatory encryption
                            let lowest_protocol =
                                min(SUPPORTED_PROTOCOL, client_parameters.client_protocol());
                            if cfg!(feature = "tls")
                                && self.config.encryption_mode
                                && lowest_protocol < PROTOCOL_2_0
                            {
                                send_fatal!(peer=format!("{}", peer); &mut stream,
                                    FatalErrorCode::InvalidInitialization,
                                    "Encryption is mandatory, must use protocol version 2.0 or later"
                                )
                            }

                            let mut inner = self.inner.lock().await;
                            let handle =
                                LockHandle::new(self.shared_lock.clone(), self.device.clone());

                            // Create new session
                            match inner.new_session(self.config, lowest_protocol, handle) {
                                Ok((session_id, session)) => {
                                    // Negotiate encryption settings
                                    let encryption_mode = self.config.encryption_mode
                                        && lowest_protocol >= PROTOCOL_2_0;
                                    let initial_encryption = self.config.initial_encryption
                                        && lowest_protocol >= PROTOCOL_2_0;

                                    let response_param = InitializeResponseParameter::new(
                                        lowest_protocol,
                                        session_id,
                                    );

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

                                    // Continue as sync session
                                    let sync_session = SyncSession::new(
                                        session,
                                        session_id,
                                        self.config,
                                        lowest_protocol,
                                        self.authenticator.clone()
                                    );
                                    return sync_session
                                        .handle_session(
                                            stream,
                                            peer,
                                            #[cfg(feature = "tls")]
                                            acceptor,
                                        )
                                        .await;
                                }
                                Err(err) => {
                                    // Send (assumed fatal) error
                                    Message::from(err).write_to(&mut stream).await?;
                                }
                            }

                            // Stop using this connection
                            return Err(io::ErrorKind::Other.into());
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
                                    send_fatal!(peer=format!("{}", peer), session_id=session_id;
                                    &mut stream, FatalErrorCode::InvalidInitialization,
                                        "Invalid session id"
                                    );
                                }
                            };

                            let s = session.clone();
                            let mut session_guard = s.lock().await;
                            let protocol = session_guard.protocol();

                            // Check if async channel has alreasy been initialized for this session
                            if session_guard.is_initialized() {
                                send_fatal!(peer=format!("{}", peer), session_id=session_id;
                                &mut stream, FatalErrorCode::InvalidInitialization,
                                    "Async session already initialized"
                                );
                            } else {
                                log::debug!(peer=format!("{}", peer), session_id=session_id; "Async initialize");

                                // Support secure connection if "tls" feature is enabled and the agreed upon protocol is >= 2.0.
                                let secure_connection = cfg!(feature = "tls")
                                    && session_guard.protocol() >= PROTOCOL_2_0;

                                // Require encryption transaction if encryption is mandatory or initially required
                                // if not, go to normal state
                                if cfg!(feature = "tls")
                                    && (self.config.initial_encryption
                                        || self.config.encryption_mode)
                                {
                                    session_guard.set_state(SessionState::EncryptionStart)
                                } else {
                                    session_guard.set_state(SessionState::Normal)
                                }

                                MessageType::AsyncInitializeResponse
                                    .message_params(
                                        AsyncInitializeResponseControl::new(secure_connection).0,
                                        AsyncInitializeResponseParameter::new(
                                            self.config.vendor_id,
                                        )
                                        .0,
                                    )
                                    .no_payload()
                                    .write_to(&mut stream)
                                    .await?;
                                drop(session_guard);

                                // Continue as async session
                                let async_session = AsyncSession::new(
                                    session,
                                    session_id,
                                    self.config,
                                    protocol,
                                );
                                return async_session
                                    .handle_session(
                                        stream,
                                        peer,
                                        #[cfg(feature = "tls")]
                                        acceptor,
                                    )
                                    .await;
                            }
                        }
                        msg => {
                            send_fatal!(peer=format!("{}", peer);
                                &mut stream, FatalErrorCode::InvalidInitialization,
                                "Unexpected message {:?} during initialization", msg.message_type
                            );
                        }
                    }
                }
                Err(err) => {
                    // Send error to client and close if fatal
                    if err.is_fatal() {
                        Message::from(err).write_to(&mut stream).await?;
                        break Err(io::ErrorKind::Other.into());
                    } else {
                        Message::from(err).write_to(&mut stream).await?;
                    }
                }
            }
        }
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
                    "Out of session ids".to_string(),
                )
                .into());
            }
        }

        Ok(self.session_id)
    }

    // Should only return Fatal errors
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
                "Maximum number of clients exceeded".to_string(),
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
