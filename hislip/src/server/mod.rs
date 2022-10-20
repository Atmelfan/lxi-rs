use std::cmp::min;
use std::collections::HashMap;
use std::io;
use std::str::from_utf8;
use std::sync::Weak;

use async_std::net::{TcpListener, ToSocketAddrs};
use async_std::sync::Arc;

use futures::task::{Spawn, SpawnExt};
use futures::{AsyncRead, AsyncWrite, AsyncWriteExt, Stream, StreamExt};
use lxi_device::lock::{LockHandle, Mutex, RemoteLockHandle, SharedLock, SpinMutex};
use lxi_device::status::Sender as StatusSender;
use lxi_device::Device;

use crate::common::errors::{Error, FatalErrorCode, NonFatalErrorCode};
use crate::common::messages::{prelude::*, send_fatal, send_nonfatal};
use crate::common::{Protocol, SUPPORTED_PROTOCOL};
use crate::server::session::{SessionState, SharedSession};
use crate::DEFAULT_DEVICE_SUBADRESS;

pub mod session;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub vendor_id: u16,
    /// Maximum server message size
    pub max_message_size: u64,
    /// Prefer overlapped data
    pub prefer_overlap: bool,
    /// Maximum allowed number of sessions
    pub max_num_sessions: usize,
    /// Short circuited "*IDN?" response.
    /// This should be set identical to what a real "*IDN?" command would return.
    pub short_idn: Option<Vec<u8>>,
}

impl ServerConfig {
    pub fn vendor_id(mut self, vendor_id: u16) -> Self {
        self.vendor_id = vendor_id;
        self
    }

    pub fn max_message_size(mut self, max_message_size: u64) -> Self {
        self.max_message_size = max_message_size;
        self
    }

    pub fn short_idn(mut self, short_idn: &[u8]) -> Self {
        self.short_idn = Some(short_idn.to_vec());
        self
    }

    pub fn max_num_sessions(mut self, max_num_sessions: usize) -> Self {
        self.max_num_sessions = max_num_sessions;
        self
    }

    pub fn prefer_overlap(mut self) -> Self {
        self.prefer_overlap = true;
        self
    }

    pub fn prefer_synchronized(mut self) -> Self {
        self.prefer_overlap = false;
        self
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            vendor_id: 0xBEEF,
            max_message_size: 1024 * 1024,
            prefer_overlap: true,
            max_num_sessions: 64,
            short_idn: None,
        }
    }
}

type DeviceMap<DEV> = HashMap<String, (Arc<SpinMutex<SharedLock>>, Arc<Mutex<DEV>>)>;

pub struct ServerBuilder<DEV> {
    config: ServerConfig,
    devices: DeviceMap<DEV>,
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

    pub fn build(self) -> Arc<Server<DEV>> {
        assert!(
            !self.devices.is_empty(),
            "Server must have one or more devices"
        );
        Server::with_config(self.config, self.devices)
    }
}

pub struct Server<DEV>
where
    DEV: Device,
{
    inner: Arc<Mutex<InnerServer<DEV>>>,
    devices: DeviceMap<DEV>,
    config: ServerConfig,
}

impl<DEV> Server<DEV>
where
    DEV: Device + Send + 'static,
{
    pub fn new(devices: DeviceMap<DEV>) -> Arc<Self> {
        let config = ServerConfig::default();
        Self::with_config(config, devices)
    }

    pub fn with_config(config: ServerConfig, devices: DeviceMap<DEV>) -> Arc<Self> {
        Arc::new(Server {
            inner: InnerServer::new(config.max_num_sessions),
            config,
            devices,
        })
    }

    /// Start accepting connections from addr
    ///
    pub async fn accept<P>(
        self: Arc<Self>,
        addr: impl ToSocketAddrs,
        mut srq: StatusSender,
        spawner: P,
    ) -> Result<(), io::Error>
    where
        P: Spawn,
    {
        let listener = TcpListener::bind(addr).await?;
        let mut incoming = listener.incoming();
        while let Some(stream) = incoming.next().await {
            let stream = stream?;
            let peer = stream.peer_addr()?;

            let s = self.clone();
            let t = srq.get_new_receiver();
            let _res = spawner.spawn(async move {
                log::info!("{peer} connected");
                let res = s.handle_session(peer.to_string(), stream, t).await;

                log::info!("{peer} disconnected: {res:?}")
            });
        }
        Ok(())
    }

    async fn handle_session<S, SRQ>(
        &self,
        peer: String,
        mut stream: S,
        srq: SRQ,
    ) -> Result<(), io::Error>
    where
        S: AsyncRead + AsyncWrite + Unpin,
        SRQ: Stream<Item = u8> + Unpin,
    {
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
                            message_type: typ @ MessageType::Error | typ @ MessageType::FatalError,
                            control_code,
                            payload,
                            ..
                        } => {
                            if typ == MessageType::FatalError {
                                log::error!(peer=peer.to_string();
                                    "Client fatal error {:?}: {}", FatalErrorCode::from_error_code(control_code),
                                    from_utf8(&payload).unwrap_or("<invalid utf8>")
                                );
                            } else {
                                log::warn!(peer=peer.to_string();
                                    "Client error {:?}: {}", NonFatalErrorCode::from_error_code(control_code),
                                    from_utf8(&payload).unwrap_or("<invalid utf8>")
                                );
                            }
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

                                    // Create new session
                                    let mut inner = self.inner.lock().await;
                                    let handle = LockHandle::new(lock.clone(), dev.clone());
                                    let session = inner.create_session(protocol, handle);
                                    drop(inner);

                                    match session {
                                        Ok((id, shared, device)) => {
                                            let response_param =
                                                InitializeResponseParameter::new(protocol, id);

                                            let control = InitializeResponseControl::new(
                                                self.config.prefer_overlap,
                                                false,
                                                false,
                                            );

                                            let receiver = {
                                                let s = shared.lock().await;
                                                s.get_clear_receiver()
                                            };

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
                                                self.config.clone(),
                                                shared,
                                                RemoteLockHandle::new(device),
                                                receiver,
                                            )
                                            .handle_session(stream, peer.clone(), protocol)
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
                                send_fatal!(peer=peer.to_string(), session_id=id;
                                &mut stream, FatalErrorCode::InvalidInitialization,
                                    "Async session already initialized"
                                );
                            } else {
                                log::debug!(peer=format!("{}", peer), session_id=id; "Async initialize");

                                session_guard.set_state(SessionState::Normal);
                                let protocol = session_guard.protocol();
                                let sender = session_guard.get_clear_sender();
                                drop(session_guard);

                                MessageType::AsyncInitializeResponse
                                    .message_params(
                                        AsyncInitializeResponseControl::new(false).0,
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
                                    self.config.clone(),
                                    shared,
                                    device,
                                    sender,
                                )
                                .handle_session(stream, peer.clone(), srq, protocol)
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
pub(crate) struct SessionHandle<DEV>
where
    DEV: Device,
{
    _id: u16,
    shared: Weak<Mutex<SharedSession>>,
    device: Weak<SpinMutex<LockHandle<DEV>>>,
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
        Self {
            _id: id,
            shared: session,
            device: handle,
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

type SessionInfo<DEV> = (Arc<Mutex<SharedSession>>, Arc<SpinMutex<LockHandle<DEV>>>);

type NewSession<DEV> = (
    u16,
    Arc<Mutex<SharedSession>>,
    Arc<SpinMutex<LockHandle<DEV>>>,
);

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
                ));
            }
        }

        Ok(self.session_id)
    }

    // Should only return Fatal errors
    fn create_session(
        &mut self,
        protocol: Protocol,
        handle: LockHandle<DEV>,
    ) -> Result<NewSession<DEV>, Error> {
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
    fn get_session(&mut self, session_id: u16) -> Option<SessionInfo<DEV>> {
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
