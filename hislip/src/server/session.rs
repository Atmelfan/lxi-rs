use async_std::sync::{Arc, Mutex};
use async_std::{
    net::{TcpListener, TcpStream, ToSocketAddrs}, // 3
    prelude::*,
    task,
};
use byteorder::{ByteOrder, NetworkEndian};
use futures::channel::mpsc;
use futures::StreamExt;

use crate::protocol::errors::{Error, FatalErrorCode};
use crate::protocol::messages::{
    AsyncInitializeResponseControl, AsyncInitializeResponseParameter, FeatureBitmap,
    InitializeParameter, InitializeResponseControl, InitializeResponseParameter, Message,
    MessageType, Protocol,
};
use crate::server;
use crate::server::ServerConfig;
use crate::Result;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SessionMode {
    Synchronized,
    Overlapped,
}

pub(crate) struct Session {
    sub_adress: String,
    /// Negotiated rpc
    protocol: Protocol,
    /// Negotiated session mode
    mode: SessionMode,
    /// Session ID
    session_id: u16,
    /// Client max message size
    max_message_size: u64,

    // Internal statekeeping between async and sync channel
    async_connected: bool,
    async_encrypted: bool,
}

type Sender<T> = mpsc::UnboundedSender<T>;
type Receiver<T> = mpsc::UnboundedReceiver<T>;

pub enum Event {
    Shutdown,
    ///
    ClearDevice,
    ///
    Data(Vec<u8>),
}

impl Session {
    pub(crate) fn new(sub_adress: String, session_id: u16, protocol: Protocol) -> Self {
        Session {
            sub_adress,
            protocol,
            mode: SessionMode::Synchronized,
            session_id,
            max_message_size: 256,
            async_connected: false,
            async_encrypted: false,
        }
    }

    pub(crate) async fn session_async_writer_loop(
        mut messages: Receiver<Event>,
        _stream: Arc<TcpStream>,
    ) -> Result<()> {
        let mut data: Vec<u8> = Vec::new();
        while let Some(event) = messages.next().await {
            match event {
                Event::Shutdown => {}
                Event::ClearDevice => {}
                Event::Data(output) => {
                    data = output;
                }
            }
        }
        Ok(())
    }

    async fn session_async_reader_loop(_stream: Arc<TcpStream>) -> Result<()> {
        let _input_buffer: Vec<u8> = Vec::new();
        let _output_buffer: Vec<u8> = Vec::new();
        //while let Ok(msg) = server::read_message_from_stream(stream.clone(), config.max_message_size).await { // 4

        //}
        Ok(())
    }

    pub(crate) async fn handle_sync_message(
        session: Arc<Mutex<Session>>,
        stream: Arc<TcpStream>,
        config: ServerConfig,
    ) -> Result<(), Error> {
        let _session_id = {
            let session_guard = session.lock().await;
            let parameter =
                InitializeResponseParameter::new(session_guard.protocol, session_guard.session_id);
            let control = InitializeResponseControl::new(
                config.preferred_mode == SessionMode::Overlapped,
                config.encryption_mandatory,
                config.secure_connection,
            );
            let response =
                MessageType::InitializeResponse.message_params(0, control.0, parameter.0);
            server::write_header_to_stream(stream.clone(), response, &[]).await?;

            session_guard.session_id
        };

        let mut command_buffer: Vec<u8> = Vec::new();
        let mut device_clear_in_progress = false;
        loop {
            match server::read_message_from_stream(stream.clone(), config.max_message_size).await {
                Ok(msg) => {
                    log::trace!("Async {:?}", msg.header);

                    match msg.header.message_type {
                        //MessageType::Initialize => {} // Already initialized
                        MessageType::FatalError => {
                            log::error!("Client fatal error: {}", str::from_utf8(msg.payload).unwrap_or("<invalid utf8>"));
                            //break; // Let client close connection
                        }
                        MessageType::Error => {
                            log::warning!("Client error: {}", str::from_utf8(msg.payload).unwrap_or("<invalid utf8>"));
                        }
                        MessageType::Data | MessageType::DataEnd => {
                            if device_clear_in_progress {
                                // Ignore any data when a device clear is in progress
                                continue;
                            } else {
                                if Ok(()) = command_buffer.try_reserve(msg.len) {
                                    command_buffer.append(msg.payload);
                                    if msg.header.message_type == MessageType::DataEnd {
                                        // Data implies END, send to application layer
                                    }
                                } else {
        
                                }
                                
                            }
                        }
                        MessageType::DeviceClearComplete => {
                            device_clear_in_progress = false;
                        }
                        MessageType::Trigger => {
                            if device_clear_in_progress {
                                // Ignore any data when a device clear is in progress
                                continue;
                            } else {
        
                            }
                        }
                        //MessageType::Interrupted => {}
                        MessageType::GetDescriptors => {}
                        MessageType::StartTLS => {}
                        MessageType::EndTLS => {}
                        MessageType::GetSaslMechanismList => {}
                        MessageType::GetSaslMechanismListResponse => {}
                        MessageType::AuthenticationStart => {}
                        MessageType::AuthenticationExchange => {}
                        MessageType::AuthenticationResult => {}
                        MessageType::VendorSpecific(code) => {
                            log::error!("Unrecognized Vendor Defined Message ({}) on sync channel", code);
                            server::write_error_to_stream(
                                stream.clone(),
                                Error::NonFatal(
                                    NonFatalErrorCode::UnrecognizedVendorDefinedMessage,
                                    b"Unrecognized Vendor Defined Message",
                                ),
                            )
                            .await?;
        
                            continue;
                        }
                        _ => {
                            log::error!("Unrecognized Message Type ({:?}) on sync channel", msg.header.message_type);
                            return Error::Fatal(
                                FatalErrorCode::InvalidInitialization,
                                b"Unrecognized Message Type",
                            );
                        }
                    }
                },
                Err(err) => {
                    if err.is_fatal() {
                        return Err(err);
                    } else {
                        write_error_to_stream(stream.clone(), err).await?;
                    }
                }
            }
        }

        Ok(())
    }

    pub(crate) async fn handle_async_connection(
        session: Arc<Mutex<Session>>,
        stream: Arc<TcpStream>,
        config: ServerConfig,
    ) -> Result<()> {
        // Sanity check
        let session_id = {
            let mut session_guard = session.lock().await;
            if session_guard.async_connected {
                log::error!("Async channel already established");
                // Session already have an async connection!
                server::write_error_to_stream(
                    stream.clone(),
                    Error::Fatal(
                        FatalErrorCode::InvalidInitialization,
                        b"Async channel already established",
                    ),
                )
                .await?;
                // Disconnect
                return Ok(());
            } else {
                session_guard.async_connected = true;
                let parameter = AsyncInitializeResponseParameter::new(config.vendor_id);
                let control = AsyncInitializeResponseControl::new(config.secure_connection);

                let response =
                    MessageType::AsyncInitializeResponse.message_params(0, control.0, parameter.0);
                server::write_header_to_stream(stream.clone(), response, &[]).await?;
            }
            session_guard.session_id
        };

        //
        while let Ok(msg) =
            server::read_message_from_stream(stream.clone(), config.max_message_size).await
        {
            log::trace!("Async {:?}", msg.header);

            match msg.header.message_type {
                MessageType::FatalError => {}
                MessageType::Error => {}
                MessageType::AsyncLock => {}
                MessageType::AsyncRemoteLocalControl => {}
                MessageType::AsyncInterrupted => {}
                MessageType::AsyncMaximumMessageSize => {
                    let mut session_guard = session.lock().await;
                    let size = NetworkEndian::read_u64(msg.payload.as_slice());
                    session_guard.max_message_size = size;
                    log::debug!("Session {}, Max client message size = {}", session_id, size);

                    let mut buf = [0u8; 8];
                    let response =
                        MessageType::AsyncMaximumMessageSizeResponse.message_params(8, 0, 0);
                    NetworkEndian::write_u64(&mut buf, config.max_message_size as u64);
                    server::write_header_to_stream(stream.clone(), response, &buf).await?;
                }
                MessageType::AsyncInitialize => {}
                MessageType::AsyncDeviceClear => {
                    let _session_guard = session.lock().await;
                    log::debug!("Session {}, Device clear", session_id);
                    let features = FeatureBitmap::new(
                        config.preferred_mode == SessionMode::Overlapped,
                        config.encryption_mandatory,
                        config.secure_connection,
                    );
                    let response =
                        MessageType::AsyncDeviceClearAcknowledge.message_params(0, features.0, 0);
                    server::write_header_to_stream(stream.clone(), response, &[]).await?;
                }
                MessageType::AsyncServiceRequest => {}
                MessageType::AsyncStatusQuery => {}
                MessageType::AsyncLockInfo => {}
                MessageType::AsyncStartTLS => {}
                MessageType::AsyncEndTLS => {}
                MessageType::VendorSpecific(_) => {}
                _ => {
                    log::error!("Unexpected message on async channel");
                    // Session already have an async connection!
                    server::write_error_to_stream(
                        stream.clone(),
                        Error::Fatal(
                            FatalErrorCode::InvalidInitialization,
                            b"Unexpected message on async channel",
                        ),
                    )
                    .await?;
                    // Disconnect
                    break;
                }
            }
        }
        Ok(())
    }
}
