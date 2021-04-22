use core::mem::drop;

use async_std::{
    net::{TcpListener, TcpStream, ToSocketAddrs}, // 3
    prelude::*,
    task,
};
use async_std::sync::{Arc, Mutex};
use byteorder::{ByteOrder, NetworkEndian};
use futures::channel::mpsc;
use futures::StreamExt;

use crate::protocol::errors::{Error, FatalErrorCode, NonFatalErrorCode};
use crate::protocol::messages::{AsyncInitializeResponseControl, AsyncInitializeResponseParameter, FeatureBitmap, InitializeParameter, InitializeResponseControl, InitializeResponseParameter, Message, MessageType, Protocol};
use crate::Result;
use crate::server;
use crate::server::{ServerConfig, write_message_to_stream};

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
    Data(Vec<u8>)
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
        stream: Arc<TcpStream>
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

    async fn session_async_reader_loop(stream: Arc<TcpStream>) -> Result<()> {
        let mut input_buffer: Vec<u8> = Vec::new();
        let mut output_buffer: Vec<u8> = Vec::new();
        //while let Ok(msg) = server::read_message_from_stream(stream.clone(), config.max_message_size).await { // 4


        //}
        Ok(())
    }


    pub(crate) async fn handle_sync_connection(
        session: Arc<Mutex<Session>>,
        stream: Arc<TcpStream>,
        config: ServerConfig,
    ) -> Result<()> {
        let session_id = {
            let mut session_guard = session.lock().await;
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

        //
        if config.secure_connection {
            let header = server::read_message_from_stream(stream.clone(), 0).await?;
            if header.header.message_type == MessageType::AsyncStartTLS {
                let mut session_guard = session.lock().await;
                if !session_guard.async_connected {
                    server::write_error_to_stream(
                        stream.clone(),
                        Error::Fatal(
                            FatalErrorCode::AttemptUseWithoutBothChannels,
                            b"Async channel not established",
                        ),
                    )
                    .await?;
                    return Ok(());
                }
                if !session_guard.async_encrypted {
                    server::write_error_to_stream(
                        stream.clone(),
                        Error::Fatal(
                            FatalErrorCode::InvalidInitialization,
                            b"Async channel not encrypted",
                        ),
                    )
                    .await?;
                    return Ok(());
                }
            }
        }

        while let Ok(msg) = server::read_message_from_stream(stream.clone(), config.max_message_size).await {
            log::trace!(
                "Async {:?}", msg.header
            );

            match msg.header.message_type {
                MessageType::Initialize => {}
                MessageType::FatalError => {}
                MessageType::Error => {}
                MessageType::Data => {}
                MessageType::DataEnd => {}
                MessageType::DeviceClearComplete => {}
                MessageType::Trigger => {}
                MessageType::Interrupted => {}
                MessageType::GetDescriptors => {}
                MessageType::StartTLS => {}
                MessageType::EndTLS => {}
                MessageType::GetSaslMechanismList => {}
                MessageType::GetSaslMechanismListResponse => {}
                MessageType::AuthenticationStart => {}
                MessageType::AuthenticationExchange => {}
                MessageType::AuthenticationResult => {}
                MessageType::VendorSpecific(_) => {}
                _ => {
                    log::error!("Unexpected message on sync channel");
                    // Session already have an async connection!
                    server::write_error_to_stream(
                        stream.clone(),
                        Error::Fatal(
                            FatalErrorCode::InvalidInitialization,
                            b"Unexpected message on sync channel",
                        ),
                    ).await?;
                    // Disconnect
                    break;
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
        while let Ok(msg) = server::read_message_from_stream(stream.clone(), config.max_message_size).await {
            log::trace!(
                "Async {:?}", msg.header
            );

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
                    let response = MessageType::AsyncMaximumMessageSizeResponse.message_params(8, 0, 0);
                    NetworkEndian::write_u64(&mut buf, config.max_message_size as u64);
                    server::write_header_to_stream(stream.clone(), response, &buf).await?;
                }
                MessageType::AsyncInitialize => {}
                MessageType::AsyncDeviceClear => {
                    let mut session_guard = session.lock().await;
                    log::debug!("Session {}, Device clear", session_id);
                    let features = FeatureBitmap::new(config.preferred_mode == SessionMode::Overlapped, config.encryption_mandatory, config.secure_connection);
                    let response = MessageType::AsyncDeviceClearAcknowledge.message_params(0, features.0, 0);
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
                    ).await?;
                    // Disconnect
                    break;
                }
            }
        }
        Ok(())
    }
}
