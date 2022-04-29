use std::cmp::min;
use std::collections::HashMap;
use std::io;
use std::str::from_utf8;

use async_rustls::server::TlsStream;
use async_std::sync::{Arc, Mutex};
use async_std::{
    net::{TcpListener, TcpStream, ToSocketAddrs}, // 3
    prelude::*,
    task,
};
use byteorder::{ByteOrder, NetworkEndian};
use futures::StreamExt;

use crate::common::Protocol;
use crate::common::errors::{Error, FatalErrorCode, NonFatalErrorCode};
use crate::common::messages::{
    AsyncInitializeResponseControl, AsyncInitializeResponseParameter, FeatureBitmap, Header,
    InitializeParameter, InitializeResponseControl, InitializeResponseParameter, Message,
    MessageType,
};
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

pub struct Server {
    inner: Arc<Mutex<InnerServer>>,
    config: ServerConfig,
}

impl Server {
    pub fn new(_vendor_id: u16) -> Arc<Server> {
        Arc::new(Server {
            inner: InnerServer::new(),
            config: ServerConfig::default(),
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

                                    // Check that
                                    let sub_adress =
                                        if let Ok(s) = String::from_utf8(msg.payload().clone()) {
                                            s
                                        } else {
                                            Message::from(Error::Fatal(
                                                FatalErrorCode::InvalidInitialization,
                                                b"Invalid sub adress",
                                            ))
                                            .write_to(&mut stream)
                                            .await?;
                                            break;
                                        };
                                    log::debug!(
                                        "Sync initialize {:?}, version={}, vendor={}",
                                        sub_adress,
                                        client_parameters.client_protocol(),
                                        client_parameters.client_vendorid()
                                    );

                                    let lowest_protocol =
                                        min(PROTOCOL_2_0, client_parameters.client_protocol());

                                    // Create new session
                                    let (session_id, session) = {
                                        let mut guard = self.inner.lock().await;
                                        match guard.new_session(sub_adress.clone(), lowest_protocol)
                                        {
                                            Ok(s) => s,
                                            Err(err) => {
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
                                    let mut session_guard = session.lock_arc().await;

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
                                    connection_state = ConnectionState::Asynchronous(session);
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

                                match others {
                                    MessageType::AsyncLock => todo!(),
                                    MessageType::AsyncRemoteLocalControl => todo!(),
                                    MessageType::AsyncInterrupted => todo!(),
                                    MessageType::AsyncMaximumMessageSize => {
                                        let size =
                                            NetworkEndian::read_u64(msg.payload().as_slice());
                                        session.max_message_size = size;
                                        log::debug!(
                                            "Session {}, Max client message size = {}",
                                            session.id,
                                            size
                                        );

                                        let mut buf = [0u8; 8];

                                        NetworkEndian::write_u64(
                                            &mut buf,
                                            self.config.max_message_size as u64,
                                        );
                                        MessageType::AsyncMaximumMessageSizeResponse
                                            .message_params(0, 0)
                                            .with_payload(buf.to_vec())
                                            .write_to(&mut stream)
                                            .await?;
                                    }
                                    MessageType::AsyncDeviceClear => {
                                        let session = s.lock().await;
                                        log::debug!("Session {}, Device clear", session.id);
                                        let features = FeatureBitmap::new(
                                            self.config.preferred_mode == SessionMode::Overlapped,
                                            self.config.encryption_mandatory,
                                            self.config.initial_encryption,
                                        );
                                        MessageType::AsyncDeviceClearAcknowledge
                                            .message_params(features.0, 0)
                                            .no_payload()
                                            .write_to(&mut stream)
                                            .await?;
                                    }
                                    MessageType::AsyncServiceRequest => todo!(),
                                    MessageType::AsyncStatusQuery => todo!(),
                                    MessageType::AsyncLockInfo => todo!(),
                                    MessageType::AsyncStartTLS => todo!(),
                                    MessageType::AsyncEndTLS => todo!(),
                                    _ => {
                                        log::error!("Unexpected message type in asynchronous channel");
                                        Message::from(Error::Fatal(
                                            FatalErrorCode::InvalidInitialization,
                                            b"Unexpected messagein asynchronous channel",
                                        ))
                                        .write_to(&mut stream)
                                        .await?;
                                        break;
                                    }
                                }
                            }
                            ConnectionState::Synchronous(s) => {
                                let mut session = s.lock().await;
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

                                    },
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
                                        log::error!("Unexpected message type in synchronous channel");
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

enum ConnectionState {
    Handshake,
    Synchronous(Arc<Mutex<Session>>),
    Asynchronous(Arc<Mutex<Session>>),
}
struct InnerServer {
    session_id: u16,
    sessions: HashMap<u16, Arc<Mutex<Session>>>,
}

impl InnerServer {
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
        sub_adress: String,
        protocol: Protocol,
    ) -> Result<(u16, Arc<Mutex<Session>>), Error> {
        let session_id = self.new_session_id()?;
        let session = Arc::new(Mutex::new(Session::new(sub_adress, session_id, protocol)));
        self.sessions.insert(session_id, session.clone());
        Ok((session_id, session))
    }
}
