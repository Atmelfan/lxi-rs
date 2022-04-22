use std::cmp::min;
use std::collections::HashMap;

use async_std::sync::{Arc, Mutex};
use async_std::{
    net::{TcpListener, TcpStream, ToSocketAddrs}, // 3
    prelude::*,
    task,
};
use futures::StreamExt;

use crate::protocol::errors::{Error, FatalErrorCode, NonFatalErrorCode};
use crate::protocol::messages::{Header, InitializeParameter, Message, MessageType, Protocol};
use crate::server::session::{Session, SessionMode};
use crate::{Result, PROTOCOL_2_0};

pub mod session;

pub enum Connection<IO> {
    Open(IO)
    Encrypted(TlsStream<IO>)
}

pub(crate) async fn read_message_from_stream(
    stream: Arc<TcpStream>,
    maxlen: usize,
) -> Result<Message, Error> {
    let mut stream = &*stream;
    let mut buf = [0u8; Header::MESSAGE_HEADER_SIZE];
    stream.read_exact(&mut buf).await.ok_or_else(|| Error::Fatal(FatalErrorCode::IoError, "Internal IO error"))?;
    let header = Header::from_buffer(&buf)?;
    if header.len > maxlen {
        Err(Error::Fatal(
            NonFatalErrorCode::MessageTooLarge,
            b"Message payload too large",
        ))
    } else {
        let mut payload = Vec::with_capacity(header.len);
        if header.len > 0 {
            stream.read_exact(payload.as_mut_slice()).ok_or_else(|| Error::Fatal(FatalErrorCode::IoError, "Internal IO error"))?;
        }

        Ok(Message { header, payload })
    }
}

pub(crate) async fn write_message_to_stream(stream: Arc<TcpStream>, msg: &Message) -> Result<()> {
    let mut stream = &*stream;
    let mut buf = [0u8; Header::MESSAGE_HEADER_SIZE];
    msg.header.pack_buffer(&mut buf);
    stream.write_all(&buf).await?;
    stream.write_all(msg.payload.as_slice()).await?;
    Ok(())
}

pub(crate) async fn write_header_to_stream(
    stream: Arc<TcpStream>,
    header: Header,
    payload: &[u8],
) -> Result<()> {
    let mut stream = &*stream;
    let mut buf = [0u8; Header::MESSAGE_HEADER_SIZE];
    header.pack_buffer(&mut buf);
    stream.write_all(&buf).await?;
    stream.write_all(payload).await?;
    Ok(())
}

pub(crate) async fn write_error_to_stream(stream: Arc<TcpStream>, error: Error) -> Result<()> {
    match error {
        Error::Fatal(code, msg) => {
            let hdr = MessageType::FatalError.message_params(msg.len(), code.error_code(), 0);
            write_header_to_stream(stream, hdr, msg).await
        }
        Error::NonFatal(code, msg) => {
            let hdr = MessageType::Error.message_params(msg.len(), code.error_code(), 0);
            write_header_to_stream(stream, hdr, msg).await
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ServerConfig {
    pub vendor_id: u16,
    /// Maximum server message size
    pub max_message_size: usize,
    pub preferred_mode: SessionMode,
    pub encryption_mandatory: bool,
    pub secure_connection: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            vendor_id: 0xBEEF,
            max_message_size: 1024,
            preferred_mode: SessionMode::Synchronized,
            encryption_mandatory: false,
            secure_connection: false,
        }
    }
}

pub struct Server {
    inner: Arc<Mutex<InnerServer>>,
    config: ServerConfig,
}

impl Server {
    pub fn new(_vendor_id: u16) -> Self {
        Server {
            inner: InnerServer::new(),
            config: ServerConfig::default(),
        }
    }

    /// Accept clients
    ///
    pub async fn accept(&self, addr: impl ToSocketAddrs) -> Result<()> {
        InnerServer::accept(self.inner.clone(), addr, self.config).await
    }
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
    fn new_session_id(&mut self) -> Result<u16> {
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
    ) -> Result<(u16, Arc<Mutex<Session>>)> {
        let session_id = self.new_session_id()?;
        let session = Arc::new(Mutex::new(Session::new(sub_adress, session_id, protocol)));
        self.sessions.insert(session_id, session.clone());
        session
    }

    /// Start accepting connections from addr
    ///
    async fn accept(
        server: Arc<Mutex<InnerServer>>,
        addr: impl ToSocketAddrs,
        config: ServerConfig,
    ) -> Result<()> {
        let listener = TcpListener::bind(addr).await?;
        let mut incoming = listener.incoming();
        while let Some(stream) = incoming.next().await {
            let stream = stream?;
            let _handle = task::spawn(Self::handle_connection(server.clone(), stream, config));
        }
        Ok(())
    }

    enum ConnectionState {
        Handshake,
        Synchronous(Session),
        Asynchronous(Session)
    }

    /// The connection handling function.
    async fn handle_connection(
        server: Arc<Mutex<InnerServer>>,
        tcp_stream: TcpStream,
        config: ServerConfig,
    ) -> Result<()> {
        let peer_addr = tcp_stream.peer_addr()?;
        log::info!("{} connected", peer_addr);

        let mut connection_state = ConnectionState::Handshake;

        // Start reading packets from stream
        let stream = Arc::new(tcp_stream);
        loop {

            match read_message_from_stream(stream.clone(), config.max_message_size).await {
                Ok(msg) => {
                    log::trace!("Received {:?}", msg.header);

                    // Handle messages
                    match msg.header.message_type {
                        MessageType::Initialize => {
                            if !matches!(connection_state, ConnectionState::Handshake) {
                                write_error_to_stream(
                                    stream.clone(),
                                    Error::Fatal(
                                        FatalErrorCode::InvalidInitialization,
                                        b"Already initialized",
                                    ),
                                )
                                .await?;
                                break;
                            }

                            if !msg.payload.is_ascii() {
                                write_error_to_stream(
                                    stream.clone(),
                                    Error::Fatal(
                                        FatalErrorCode::InvalidInitialization,
                                        b"Invalid sub-adress",
                                    ),
                                )
                                .await?;
                                break;
                            }

                            // Create new session
                            let client_parameters = InitializeParameter(msg.message_parameter());

                            // Check that
                            let sub_adress = String::from_utf8(msg.payload).unwrap();
                            log::debug!(
                                "Sync initialize {:?},rpc={},vendor={}",
                                sub_adress,
                                client_parameters.client_protocol(),
                                client_parameters.client_vendorid()
                            );

                            let lowest_protocol =
                                min(PROTOCOL_2_0, client_parameters.client_protocol());

                            // Create new session
                            let (session_id, session) = {
                                let guard = server.lock().await?;
                                guard.new_session(sub_adress.clone(), lowest_protocol)?
                            };
                            log::debug!("New session 0x{:04x}", session_id);

                            // Send response
                            let response_param = InitializeResponseParameter::new(lowest_protocol, session_id);
                            let control = InitializeResponseControl::new(
                                config.preferred_mode == SessionMode::Overlapped,
                                config.encryption_mandatory,
                                config.secure_connection,
                            );
                            let response = MessageType::InitializeResponse.message_params(0, control.0, parameter.0);
                            write_header_to_stream(stream.clone(), response, &[]).await?;

                            // Connection is a synchronous channel
                            connection_state = ConnectionState::Synchronous(session);
                        }
                        MessageType::AsyncInitialize => {
                            

                            if !matches!(connection_state, ConnectionState::Handshake) {
                                write_error_to_stream(
                                    stream.clone(),
                                    Error::Fatal(
                                        FatalErrorCode::InvalidInitialization,
                                        b"Already initialized",
                                    ),
                                )
                                .await?;
                                break;
                            }

                            // Connect to existing session
                            let session_id = (msg.message_parameter() & 0x0000FFFF) as u16;
                            let session = {
                                let guard = server.lock().await;
                                if let Some(s) = guard.sessions.get(session_id).cloned() {
                                    s
                                } else {
                                    write_error_to_stream(
                                        stream.clone(),
                                        Error::Fatal(
                                            FatalErrorCode::InvalidInitialization,
                                            b"Invalid session id",
                                        ),
                                    )
                                    .await?;
                                    break;
                                }
                            };

                            log::debug!("AsyncInitialize session=0x{:04x}", session_id);

                            session_guard.async_connected = true;
                            let parameter = AsyncInitializeResponseParameter::new(config.vendor_id);
                            let control = AsyncInitializeResponseControl::new(config.secure_connection);
            
                            let response = MessageType::AsyncInitializeResponse.message_params(0, control.0, parameter.0);
                            server::write_header_to_stream(stream.clone(), response, &[]).await?;
                                
                            connection_state = ConnectionState::Asynchronous(session);
                        },
                        MessageType::VendorSpecific(code) => {
                            log::error!("Unrecognized Vendor Defined Message ({}) during init", code);
                            server::write_error_to_stream(
                                stream.clone(),
                                Error::NonFatal(
                                    NonFatalErrorCode::UnrecognizedVendorDefinedMessage,
                                    b"Unrecognized Vendor Defined Message",
                                ),
                            )
                            .await?;
                        },
                        MessageType::FatalError => {
                            log::error!("Client fatal error: {}", str::from_utf8(msg.payload).unwrap_or("<invalid utf8>"));
                            //break; // Let client close connection
                        },
                        MessageType::Error => {
                            log::warning!("Client error: {}", str::from_utf8(msg.payload).unwrap_or("<invalid utf8>"));
                        },
                        // Synchronous messages
                        _ if matches!(connection_state, ConnectionState::Synchronous(...)) => {

                        },
                        // Asynchronous messages
                        _ if matches!(connection_state, ConnectionState::Asynchronous(...)) => {

                        },
                        // Other messages shouldn't occur unless initialized
                        _ => {
                            log::error!("Unexpected message type");
                            write_error_to_stream(
                                stream.clone(),
                                Error::Fatal(
                                    FatalErrorCode::InvalidInitialization,
                                    b"Unexpected message",
                                ),
                            )
                            .await?;
                            break;
                        }
                    }
                },
                Err(err) => {
                    write_error_to_stream(stream.clone(), err).await?;
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




