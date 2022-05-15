use std::io;
use std::time::Duration;

use async_std::future;
use async_std::net::TcpStream;
use async_std::sync::Arc;
use byteorder::{ByteOrder, NetworkEndian};
use futures::channel::mpsc;
use futures::StreamExt;
use lxi_device::lock::{LockHandle, SharedLockError, SharedLockMode};

use crate::common::errors::{Error, FatalErrorCode};
use crate::common::messages::{FeatureBitmap, Message, MessageType};
use crate::common::Protocol;

use super::ServerConfig;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SessionMode {
    Synchronized,
    Overlapped,
}

pub(crate) struct Session<DEV> {
    config: ServerConfig,
    /// Negotiated rpc
    pub(crate) protocol: Protocol,
    /// Negotiated session mode
    pub(crate) mode: SessionMode,
    /// Session ID
    pub(crate) id: u16,
    /// Client max message size
    pub(crate) max_message_size: u64,

    // Internal statekeeping between async and sync channel
    pub(crate) async_connected: bool,
    pub(crate) async_encrypted: bool,

    pub(crate) handle: LockHandle<DEV>,

    // Input/Output buffer
    pub(crate) in_buf: Vec<u8>,
    pub(crate) out_buf: Vec<u8>,
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

impl<DEV> Session<DEV> {
    pub(crate) fn new(
        config: ServerConfig,
        session_id: u16,
        protocol: Protocol,
        handle: LockHandle<DEV>,
    ) -> Self {
        Self {
            config,
            protocol,
            mode: SessionMode::Synchronized,
            id: session_id,
            max_message_size: 256,
            async_connected: false,
            async_encrypted: false,
            handle,
            in_buf: Vec::new(),
            out_buf: Vec::new(),
        }
    }

    pub(crate) fn close(&mut self) {
        // Release any lock this session might be holding
        // Should be called anyways by LockHandle::drop() but done here to be obvious
        self.handle.force_release();
    }

    pub(crate) async fn session_async_writer_loop(
        mut messages: Receiver<Event>,
        _stream: Arc<TcpStream>,
    ) -> Result<(), Error> {
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

    pub(crate) async fn handle_sync_message(
        self: &mut Self,
        msg: Message,
        stream: &mut TcpStream,
    ) -> Result<(), io::Error> {
        todo!()
    }

    pub(crate) async fn handle_async_message(
        self: &mut Self,
        msg: Message,
        stream: &mut TcpStream,
    ) -> Result<(), io::Error> {
        match msg.message_type() {
            MessageType::AsyncLock => {
                if msg.control_code() == 0 {
                    // Release
                    let message_id = msg.message_parameter();
                    log::debug!(session_id = self.id, message_id = message_id; "Release async lock");
                    let control = match self.handle.try_release() {
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
                        log::error!(session_id = self.id, timeout=timeout; "Lockstr is not valid UTF8");
                        Message::from(Error::Fatal(
                            FatalErrorCode::UnidentifiedError,
                            b"Lockstr is not valid UTF8",
                        ))
                        .write_to(stream)
                        .await?;
                        return Err(io::ErrorKind::Other.into());
                    };
                    log::debug!(session_id = self.id, timeout=timeout; "Async lock: '{}'", lockstr);

                    // Try to acquire lock
                    let res = if timeout == 0 {
                        // Try to lock immediately
                        if lockstr.is_empty() {
                            self.handle.try_acquire_exclusive()
                        } else {
                            self.handle.try_acquire_shared(lockstr)
                        }
                    } else {
                        // Try to lock until timed out
                        if lockstr.is_empty() {
                            future::timeout(
                                Duration::from_millis(timeout as u64),
                                self.handle.async_acquire_exclusive(),
                            )
                            .await
                            .map_err(|_| SharedLockError::Timeout)
                            .and_then(|res| res)
                        } else {
                            future::timeout(
                                Duration::from_millis(timeout as u64),
                                self.handle.async_acquire_shared(lockstr),
                            )
                            .await
                            .map_err(|_| SharedLockError::Timeout)
                            .and_then(|res| res)
                        }
                    };

                    //log::debug!(session_id = self.id; "Async lock: {:?}", res);

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
                self.max_message_size = size;
                log::debug!(session_id = self.id; "Max client message size = {}", size);

                let mut buf = [0u8; 8];

                NetworkEndian::write_u64(&mut buf, self.config.max_message_size as u64);
                MessageType::AsyncMaximumMessageSizeResponse
                    .message_params(0, 0)
                    .with_payload(buf.to_vec())
                    .write_to(stream)
                    .await?;
            }
            MessageType::AsyncDeviceClear => {
                log::debug!(session_id = self.id; "Device clear");
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
                let (exclusive, num_shared) = self.handle.lock_info();

                log::debug!(session_id = self.id; "Lock info, exclusive={}, shared={}", exclusive, num_shared);

                MessageType::AsyncLockInfoResponse
                    .message_params(exclusive.into(), num_shared)
                    .no_payload()
                    .write_to(stream)
                    .await?;
            }
            MessageType::AsyncStartTLS => todo!(),
            MessageType::AsyncEndTLS => todo!(),
            _ => {
                log::debug!(session_id = self.id; "Unexpected message type in asynchronous channel");
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
}

impl<DEV> Drop for Session<DEV> {
    fn drop(&mut self) {
        self.close()
    }
}
