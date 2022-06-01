use std::cmp::min;
use std::collections::HashMap;
use std::io;
use std::str::from_utf8;
use std::sync::Weak;

use async_std::channel::{self, Receiver, Sender};
use async_std::sync::Arc;
use async_std::{
    net::{TcpListener, TcpStream, ToSocketAddrs},
    task,
};

use futures::{AsyncWriteExt, StreamExt};
use lxi_device::lock::{LockHandle, Mutex, RemoteLockHandle, SharedLock, SpinMutex};
use lxi_device::Device;

use crate::common::errors::{Error, FatalErrorCode, NonFatalErrorCode};
use crate::common::messages::{prelude::*, send_fatal, send_nonfatal};
use crate::common::{Protocol, PROTOCOL_2_0, SUPPORTED_PROTOCOL};
use crate::server::session::{SessionState, SharedSession};
use crate::server::stream::HislipStream;
use crate::DEFAULT_DEVICE_SUBADRESS;

#[cfg(feature = "tls")]
use async_tls::TlsAcceptor;

pub mod auth;
use auth::Auth;

use self::auth::AnonymousAuth;

pub mod session;
mod stream;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EncryptionMode {
    Optional,
    Mandatory,
}

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
            max_message_size: 1024 * 1024,
            prefer_overlap: true,
            encryption_mode: false,
            initial_encryption: false,
            max_num_sessions: 64,
        }
    }
}

pub struct ServerBuilder<DEV> {
    config: ServerConfig,
    devices: HashMap<String, (Arc<SpinMutex<SharedLock>>, Arc<Mutex<DEV>>)>,
}

impl<DEV> Default for ServerBuilder<DEV> {
    fn default() -> Self {
        Self {
            config: Default::default(),
            devices: HashMap::new(),
        }
    }
}

impl<DEV> ServerBuilder<DEV>
where
    DEV: Device + Send + 'static,
{
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            devices: HashMap::new(),
        }
    }

    pub fn new_with_device(
        config: ServerConfig,
        subaddr: String,
        dev: Arc<Mutex<DEV>>,
        shared_lock: Arc<SpinMutex<SharedLock>>,
    ) -> Self {
        Self::new(config).device(subaddr, dev, shared_lock)
    }

    pub fn device(
        mut self,
        subaddr: String,
        dev: Arc<Mutex<DEV>>,
        shared_lock: Arc<SpinMutex<SharedLock>>,
    ) -> Self {
        self.devices.insert(subaddr, (shared_lock, dev));
        self
    }

    pub fn build_with_auth<A>(self, authenticator: Arc<Mutex<A>>) -> Arc<Server<DEV, A>>
    where
        A: Auth + Send + 'static,
    {
        assert!(
            self.devices.len() > 0,
            "Server must have one or more devices"
        );
        Server::with_config(self.config, self.devices, authenticator)
    }

    pub fn build(self) -> Arc<Server<DEV, AnonymousAuth>> {
        let authenticator = Arc::new(Mutex::new(AnonymousAuth));
        assert!(
            self.devices.len() > 0,
            "Server must have one or more devices"
        );
        Server::with_config(self.config, self.devices, authenticator)
    }
}

pub struct Server<DEV, A>
where
    DEV: Device,
    A: Auth,
{
    inner: Arc<Mutex<InnerServer<DEV>>>,
    devices: HashMap<String, (Arc<SpinMutex<SharedLock>>, Arc<Mutex<DEV>>)>,
    config: ServerConfig,
    authenticator: Arc<Mutex<A>>,
}

impl<DEV, A> Server<DEV, A>
where
    DEV: Device + Send + 'static,
    A: Auth + Send + 'static,
{
    pub fn new(
        devices: HashMap<String, (Arc<SpinMutex<SharedLock>>, Arc<Mutex<DEV>>)>,
        authenticator: Arc<Mutex<A>>,
    ) -> Arc<Self> {
        let config = ServerConfig::default();
        Arc::new(Server {
            inner: InnerServer::new(config.max_num_sessions),
            config,
            authenticator,
            devices,
        })
    }

    pub fn with_config(
        config: ServerConfig,
        devices: HashMap<String, (Arc<SpinMutex<SharedLock>>, Arc<Mutex<DEV>>)>,
        authenticator: Arc<Mutex<A>>,
    ) -> Arc<Self> {
        Arc::new(Server {
            inner: InnerServer::new(config.max_num_sessions),
            config,
            devices,
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
                log::info!("{peer} connected");

                let res = s
                    .handle_session(
                        stream,
                        #[cfg(feature = "tls")]
                        acceptor,
                    )
                    .await;

                log::info!("{peer} disconnected: {res:?}")
            });
        }
        Ok(())
    }

    async fn handle_session(
        &self,
        stream: TcpStream,
        #[cfg(feature = "tls")] acceptor: TlsAcceptor,
    ) -> Result<(), io::Error> {
        let peer = stream.peer_addr()?;

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
                            log::debug!(peer=peer.to_string();
                                "Sync initialize, version={}, vendor={}",
                                client_parameters.client_protocol(),
                                client_parameters.client_vendorid()
                            );

                            // TODO: Accept other than hislip0
                            if let Ok(mut s) = String::from_utf8(payload) {
                                if s.is_empty() {
                                    log::debug!(peer=peer.to_string(); "Empty sub-address, using default: {DEFAULT_DEVICE_SUBADRESS:?}");
                                    s = DEFAULT_DEVICE_SUBADRESS.to_string();
                                }

                                if let Some((lock, dev)) = self.devices.get(&s) {
                                    // Check if negotiated protocol is compatible with mandatory encryption
                                    let protocol = min(
                                        SUPPORTED_PROTOCOL,
                                        client_parameters.client_protocol(),
                                    );
                                    if cfg!(feature = "tls")
                                        && self.config.encryption_mode
                                        && protocol < PROTOCOL_2_0
                                    {
                                        send_fatal!(peer=format!("{}", peer); &mut stream,
                                            FatalErrorCode::InvalidInitialization,
                                            "Encryption is mandatory, must use protocol version 2.0 or later"
                                        )
                                    }

                                    let mut inner = self.inner.lock().await;
                                    let handle = LockHandle::new(lock.clone(), dev.clone());

                                    // Create new session
                                    match inner.create_session(protocol, handle) {
                                        Ok((id, shared, device)) => {
                                            // Negotiate encryption settings
                                            let encryption_mode = self.config.encryption_mode
                                                && protocol >= PROTOCOL_2_0;
                                            let initial_encryption = self.config.initial_encryption
                                                && protocol >= PROTOCOL_2_0;

                                            let response_param =
                                                InitializeResponseParameter::new(protocol, id);

                                            let control = InitializeResponseControl::new(
                                                self.config.prefer_overlap,
                                                encryption_mode,
                                                initial_encryption,
                                            );
                                            drop(inner);

                                            // Send response
                                            log::debug!(peer=peer.to_string(); "New session {id}, subaddr: {s:?}");
                                            MessageType::InitializeResponse
                                                .message_params(control.0, response_param.0)
                                                .no_payload()
                                                .write_to(&mut stream)
                                                .await?;

                                            // Continue as sync session
                                            let res = session::synchronous::SyncSession::new(
                                                id,
                                                self.config,
                                                shared,
                                                RemoteLockHandle::new(device),
                                                self.authenticator.clone(),
                                            )
                                            .await
                                            .handle_session(
                                                stream,
                                                peer,
                                                #[cfg(feature = "tls")]
                                                acceptor,
                                            )
                                            .await;
                                            log::debug!(peer=peer.to_string(), session_id=id; "Sync session closed: {res:?}");
                                            return res;
                                        }
                                        Err(err) => {
                                            // Send (assumed fatal) error
                                            Message::from(err).write_to(&mut stream).await?;
                                        }
                                    }

                                    // Stop using this connection
                                    return Err(io::ErrorKind::Other.into());
                                } else {
                                    send_fatal!(peer=format!("{}", peer); &mut stream, FatalErrorCode::InvalidInitialization, "Invalid subadress: {s}")
                                }
                            } else {
                                send_fatal!(peer=format!("{}", peer); &mut stream, FatalErrorCode::InvalidInitialization, "Invalid subadress: <invalid utf8>")
                            }
                        }
                        Message {
                            message_type: MessageType::AsyncInitialize,
                            message_parameter,
                            ..
                        } => {
                            // Connect to existing session
                            let id = (message_parameter & 0x0000FFFF) as u16;
                            let session = {
                                let mut guard = self.inner.lock().await;
                                if let Some(s) = guard.get_session(id) {
                                    s
                                } else {
                                    send_fatal!(peer=format!("{}", peer), session_id=id;
                                    &mut stream, FatalErrorCode::InvalidInitialization,
                                        "Invalid session id"
                                    );
                                }
                            };

                            let (shared, device) = session.clone();
                            let mut session_guard = shared.lock().await;

                            // Check if async channel has alreasy been initialized for this session
                            if session_guard.is_initialized() {
                                drop(session_guard);
                                send_fatal!(peer=format!("{}", peer), session_id=id;
                                &mut stream, FatalErrorCode::InvalidInitialization,
                                    "Async session already initialized"
                                );
                            } else {
                                log::debug!(peer=format!("{}", peer), session_id=id; "Async initialize");

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
                                drop(session_guard);

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

                                // Continue as async session
                                let res = session::asynchronous::AsyncSession::new(
                                    id,
                                    self.config,
                                    shared,
                                    device,
                                )
                                .await
                                .handle_session(
                                    stream,
                                    peer,
                                    #[cfg(feature = "tls")]
                                    acceptor,
                                )
                                .await;
                                log::debug!(peer=peer.to_string(), session_id=id; "Async session closed: {res:?}");
                                return res;
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

/// A handle to a created active season
#[derive(Clone)]
struct SessionHandle<DEV>
where
    DEV: Device,
{
    id: u16,
    shared: Weak<Mutex<SharedSession>>,
    device: Weak<SpinMutex<LockHandle<DEV>>>,
    clear_sender: Sender<()>,
    clear_receiver: Receiver<()>,
}

impl<DEV> SessionHandle<DEV>
where
    DEV: Device,
{
    fn new(
        id: u16,
        session: Weak<Mutex<SharedSession>>,
        handle: Weak<SpinMutex<LockHandle<DEV>>>,
    ) -> Self {
        let (clear_sender, clear_receiver) = channel::bounded(1);
        Self {
            id,
            shared: session,
            device: handle,
            clear_sender,
            clear_receiver,
        }
    }

    /// Return false if the assosciated object have been closed
    fn active(&self) -> bool {
        self.shared.strong_count() > 0 && self.device.strong_count() > 0
    }
}

struct InnerServer<DEV>
where
    DEV: Device,
{
    session_id: u16,
    sessions: HashMap<u16, SessionHandle<DEV>>,
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
    fn create_session(
        &mut self,
        protocol: Protocol,
        handle: LockHandle<DEV>,
    ) -> Result<
        (
            u16,
            Arc<Mutex<SharedSession>>,
            Arc<SpinMutex<LockHandle<DEV>>>,
        ),
        Error,
    > {
        self.gc_sessions();
        if self.sessions.len() >= self.max_num_sessions {
            return Err(Error::Fatal(
                FatalErrorCode::MaximumClientsExceeded,
                "Maximum number of clients exceeded".to_string(),
            ));
        }

        let id = self.new_session_id()?;

        // Create new resources for session
        let shared = Arc::new(Mutex::new(SharedSession::new(protocol)));
        let device = Arc::new(SpinMutex::new(handle));
        let session = SessionHandle::new(id, Arc::downgrade(&shared), Arc::downgrade(&device));

        self.sessions.insert(id, session);
        Ok((id, shared, device))
    }

    /// Get a session
    /// Note: Returns a strong reference which will keep any locks assosciated with session active until dropped
    fn get_session(
        &mut self,
        session_id: u16,
    ) -> Option<(Arc<Mutex<SharedSession>>, Arc<SpinMutex<LockHandle<DEV>>>)> {
        let tmp = self.sessions.get(&session_id)?;
        let shared = tmp.shared.upgrade()?;
        let dev = tmp.device.upgrade()?;

        Some((shared, dev))
    }

    /// Remove any stale session id
    fn gc_sessions(&mut self) {
        self.sessions.retain(|_, session| session.active())
    }
}
