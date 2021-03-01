use core::mem::drop;

use async_std::sync::{Arc, Mutex};
use async_std::{
    net::{TcpListener, TcpStream, ToSocketAddrs}, // 3
    prelude::*,
    task,
};

use crate::errors::{Error, FatalErrorCode, NonFatalErrorCode};
use crate::messages::{
    AsyncInitializeResponseControl, AsyncInitializeResponseParameter, InitializeParameter,
    InitializeResponseControl, InitializeResponseParameter, Protocol,
};
use crate::server;
use crate::server::ServerConfig;
use crate::{messages::MessageType, Result};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SessionMode {
    Synchronized,
    Overlapped,
}

pub struct Session {
    sub_adress: String,
    protocol: Protocol,
    mode: SessionMode,
    session_id: u16,
    max_message_size: u64,
    async_connected: bool,
    async_encrypted: bool,
}

impl Session {
    pub(crate) fn new(sub_adress: String, session_id: u16, protocol: Protocol) -> Self {
        Session {
            sub_adress,
            protocol,
            mode: SessionMode::Synchronized,
            session_id,
            max_message_size: 0,
            async_connected: false,
            async_encrypted: false,
        }
    }

    pub(crate) async fn handle_sync_connection(
        session: Arc<Mutex<Session>>,
        stream: Arc<TcpStream>,
        config: ServerConfig,
    ) -> Result<()> {
        {
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
        }

        //
        if config.secure_connection {
            let header = server::read_header_from_stream(stream.clone()).await?;
            if header.message_type == MessageType::AsyncStartTLS {
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

        Ok(())
    }

    pub(crate) async fn handle_async_connection(
        session: Arc<Mutex<Session>>,
        stream: Arc<TcpStream>,
        config: ServerConfig,
    ) -> Result<()> {
        // Sanity check
        {
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
                return Ok(());
            } else {
                session_guard.async_connected = true;
                let parameter = AsyncInitializeResponseParameter::new(config.vendor_id);
                let control = AsyncInitializeResponseControl::new(config.secure_connection);

                let response =
                    MessageType::AsyncInitializeResponse.message_params(0, control.0, parameter.0);
                server::write_header_to_stream(stream.clone(), response, &[]).await?;
            }
        }

        //
        while let Ok(header) = server::read_header_from_stream(stream.clone()).await {
            log::trace!(
                "Async {:?},ctrl={},par={},len={}",
                header.message_type,
                header.control_code,
                header.message_parameter,
                header.len
            );

            // Read a payload if any
            let mut payload = vec![0u8; header.len];
            if header.len > 0 {
                let mut stream = &*stream;
                stream.read_exact(&mut payload[..]).await?;
            }
        }
        Ok(())
    }
}
