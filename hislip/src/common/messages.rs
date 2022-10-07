use core::option::Option;
use core::result::Result;
use std::{fmt::Display, io};

use bitfield::bitfield;

use byteorder::{BigEndian, ByteOrder, NetworkEndian};
use lxi_device::lock::SharedLockError;

use crate::common::errors::{Error, FatalErrorCode, NonFatalErrorCode};
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use super::Protocol;

pub(crate) mod prelude {
    pub(crate) use super::{
        AsyncInitializeResponseControl, AsyncInitializeResponseParameter, FeatureBitmap,
        InitializeParameter, InitializeResponseControl, InitializeResponseParameter, Message,
        MessageType, ReleaseLockControl, RequestLockControl, RmtDeliveredControl,
    };
}

#[derive(Debug, Clone)]
pub(crate) struct Message {
    pub(crate) message_type: MessageType,
    pub(crate) control_code: u8,
    pub(crate) message_parameter: u32,
    pub(crate) payload: Vec<u8>,
}

impl Message {
    pub const MESSAGE_HEADER_SIZE: usize = 16;

    pub(crate) fn with_payload(self, payload: Vec<u8>) -> Self {
        Self { payload, ..self }
    }

    pub(crate) fn no_payload(self) -> Message {
        Self {
            payload: Vec::new(),
            ..self
        }
    }

    pub(crate) async fn read_from<RD>(
        reader: &mut RD,
        maxlen: u64,
    ) -> Result<Result<Message, Error>, io::Error>
    where
        RD: AsyncRead + Unpin,
    {
        let mut buf = [0u8; Message::MESSAGE_HEADER_SIZE];
        reader.read_exact(&mut buf).await?;
        let prolog = &buf[0..2];
        if prolog != b"HS" {
            return Ok(Err(Error::Fatal(
                FatalErrorCode::PoorlyFormattedMessageHeader,
                "Invalid prologue".to_string(),
            )));
        }

        let control_code = buf[3];
        let len = BigEndian::read_u64(&buf[8..16]);
        let message_parameter = BigEndian::read_u32(&buf[4..8]);

        if len > maxlen {
            Ok(Err(Error::NonFatal(
                NonFatalErrorCode::MessageTooLarge,
                "Message payload too large".to_string(),
            )))
        } else {
            let mut payload = Vec::with_capacity(len as usize);
            reader.take(len).read_to_end(&mut payload).await?;
            match MessageType::from_message_type(buf[2]).ok_or(Error::NonFatal(
                NonFatalErrorCode::UnrecognizedMessageType,
                "Unrecognized message type".to_string(),
            )) {
                Ok(message_type) => Ok(Ok(Message {
                    message_type,
                    control_code,
                    message_parameter,
                    payload,
                })),
                Err(err) => Ok(Err(err)),
            }
        }
    }

    pub(crate) async fn write_to<WR>(&self, writer: &mut WR) -> Result<(), io::Error>
    where
        WR: AsyncWrite + Unpin,
    {
        let mut buf = [0u8; Message::MESSAGE_HEADER_SIZE];
        buf[0] = b'H';
        buf[1] = b'S';
        buf[2] = self.message_type.get_message_type();
        buf[3] = self.control_code;
        NetworkEndian::write_u32(&mut buf[4..8], self.message_parameter);
        NetworkEndian::write_u64(&mut buf[8..16], self.payload.len() as u64);
        let mut to_send = buf.to_vec();
        to_send.extend_from_slice(&self.payload);
        writer.write_all(&to_send).await?;
        Ok(())
    }
}

impl From<Error> for Message {
    fn from(err: Error) -> Self {
        match err {
            Error::Fatal(code, msg) => MessageType::FatalError
                .message_params(code.error_code(), 0)
                .with_payload(msg.into_bytes()),
            Error::NonFatal(code, msg) => MessageType::Error
                .message_params(code.error_code(), 0)
                .with_payload(msg.into_bytes()),
        }
    }
}

macro_rules! send_fatal {
    ($stream:expr, $err:expr, $($arg:tt)*) => {{
        log::error!($($arg)*);
        Message::from(Error::Fatal($err, format!($($arg)*)))
            .write_to($stream)
            .await?;
        $stream.flush().await?;
        return Err(io::ErrorKind::Other.into());
    }};
    ($($key:ident=$value:expr),*; $stream:expr, $err:expr, $($arg:tt)*) => {{
        log::error!($($key=$value),*; $($arg)*);
        Message::from(Error::Fatal($err, format!($($arg)*)))
            .write_to($stream)
            .await?;
        $stream.flush().await?;
        return Err(io::ErrorKind::Other.into());
    }};
}
pub(crate) use send_fatal;

macro_rules! send_nonfatal {
    ($stream:expr, $err:expr, $($arg:tt)*) => {{
        log::warn!($($arg)*);
        Message::from(Error::NonFatal($err, format!($($arg)*)))
            .write_to($stream)
            .await?;
        $stream.flush().await?;
    }};
    ($($key:ident=$value:expr),*; $stream:expr, $err:expr, $($arg:tt)*) => {{
        log::warn!($($key=$value),*; $($arg)*);
        Message::from(Error::NonFatal($err, format!($($arg)*)))
            .write_to($stream)
            .await?;
        $stream.flush().await?;
    }};
}
pub(crate) use send_nonfatal;

/// Message Type Value Definitions
///
/// See Table 4 in HiSLIP specification
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum MessageType {
    Initialize,
    InitializeResponse,
    FatalError,
    Error,
    AsyncLock,
    AsyncLockResponse,
    Data,
    DataEnd,
    DeviceClearComplete,
    DeviceClearAcknowledge,
    AsyncRemoteLocalControl,
    AsyncRemoteLocalResponse,
    Trigger,
    Interrupted,
    AsyncInterrupted,
    AsyncMaximumMessageSize,
    AsyncMaximumMessageSizeResponse,
    AsyncInitialize,
    AsyncInitializeResponse,
    AsyncDeviceClear,
    AsyncServiceRequest,
    AsyncStatusQuery,
    AsyncStatusResponse,
    AsyncDeviceClearAcknowledge,
    AsyncLockInfo,
    AsyncLockInfoResponse,
    GetDescriptors,
    GetDescriptorsResponse,
    StartTLS,
    AsyncStartTLS,
    AsyncStartTLSResponse,
    EndTLS,
    AsyncEndTLS,
    AsyncEndTLSResponse,
    GetSaslMechanismList,
    GetSaslMechanismListResponse,
    AuthenticationStart,
    AuthenticationExchange,
    AuthenticationResult,
    /// Vendor-specific, only codes 128-255 are allowed
    VendorSpecific(u8),
}

impl MessageType {
    pub fn get_message_type(&self) -> u8 {
        match self {
            MessageType::Initialize => 0,
            MessageType::InitializeResponse => 1,
            MessageType::FatalError => 2,
            MessageType::Error => 3,
            MessageType::AsyncLock => 4,
            MessageType::AsyncLockResponse => 5,
            MessageType::Data => 6,
            MessageType::DataEnd => 7,
            MessageType::DeviceClearComplete => 8,
            MessageType::DeviceClearAcknowledge => 9,
            MessageType::AsyncRemoteLocalControl => 10,
            MessageType::AsyncRemoteLocalResponse => 11,
            MessageType::Trigger => 12,
            MessageType::Interrupted => 13,
            MessageType::AsyncInterrupted => 14,
            MessageType::AsyncMaximumMessageSize => 15,
            MessageType::AsyncMaximumMessageSizeResponse => 16,
            MessageType::AsyncInitialize => 17,
            MessageType::AsyncInitializeResponse => 18,
            MessageType::AsyncDeviceClear => 19,
            MessageType::AsyncServiceRequest => 20,
            MessageType::AsyncStatusQuery => 21,
            MessageType::AsyncStatusResponse => 22,
            MessageType::AsyncDeviceClearAcknowledge => 23,
            MessageType::AsyncLockInfo => 24,
            MessageType::AsyncLockInfoResponse => 25,
            MessageType::GetDescriptors => 26,
            MessageType::GetDescriptorsResponse => 27,
            MessageType::StartTLS => 28,
            MessageType::AsyncStartTLS => 29,
            MessageType::AsyncStartTLSResponse => 30,
            MessageType::EndTLS => 31,
            MessageType::AsyncEndTLS => 32,
            MessageType::AsyncEndTLSResponse => 33,
            MessageType::GetSaslMechanismList => 34,
            MessageType::GetSaslMechanismListResponse => 35,
            MessageType::AuthenticationStart => 36,
            MessageType::AuthenticationExchange => 37,
            MessageType::AuthenticationResult => 38,
            MessageType::VendorSpecific(x) => x & 0x7F,
        }
    }

    pub fn from_message_type(typ: u8) -> Option<MessageType> {
        match typ {
            0 => Some(MessageType::Initialize),
            1 => Some(MessageType::InitializeResponse),
            2 => Some(MessageType::FatalError),
            3 => Some(MessageType::Error),
            4 => Some(MessageType::AsyncLock),
            5 => Some(MessageType::AsyncLockResponse),
            6 => Some(MessageType::Data),
            7 => Some(MessageType::DataEnd),
            8 => Some(MessageType::DeviceClearComplete),
            9 => Some(MessageType::DeviceClearAcknowledge),
            10 => Some(MessageType::AsyncRemoteLocalControl),
            11 => Some(MessageType::AsyncRemoteLocalResponse),
            12 => Some(MessageType::Trigger),
            13 => Some(MessageType::Interrupted),
            14 => Some(MessageType::AsyncInterrupted),
            15 => Some(MessageType::AsyncMaximumMessageSize),
            16 => Some(MessageType::AsyncMaximumMessageSizeResponse),
            17 => Some(MessageType::AsyncInitialize),
            18 => Some(MessageType::AsyncInitializeResponse),
            19 => Some(MessageType::AsyncDeviceClear),
            20 => Some(MessageType::AsyncServiceRequest),
            21 => Some(MessageType::AsyncStatusQuery),
            22 => Some(MessageType::AsyncStatusResponse),
            23 => Some(MessageType::AsyncDeviceClearAcknowledge),
            24 => Some(MessageType::AsyncLockInfo),
            25 => Some(MessageType::AsyncLockInfoResponse),
            26 => Some(MessageType::GetDescriptors),
            27 => Some(MessageType::GetDescriptorsResponse),
            28 => Some(MessageType::StartTLS),
            29 => Some(MessageType::AsyncStartTLS),
            30 => Some(MessageType::AsyncStartTLSResponse),
            31 => Some(MessageType::EndTLS),
            32 => Some(MessageType::AsyncEndTLS),
            33 => Some(MessageType::AsyncEndTLSResponse),
            34 => Some(MessageType::GetSaslMechanismList),
            35 => Some(MessageType::GetSaslMechanismListResponse),
            36 => Some(MessageType::AuthenticationStart),
            37 => Some(MessageType::AuthenticationExchange),
            38 => Some(MessageType::AuthenticationResult),
            128..=255 => Some(MessageType::VendorSpecific(typ)),
            _ => None,
        }
    }

    pub(crate) fn message_params(self, control_code: u8, message_parameter: u32) -> Message {
        Message {
            message_type: self,
            control_code,
            message_parameter,
            payload: Vec::new(),
        }
    }
}

bitfield! {
    pub struct InitializeParameter(u32);
    impl Debug;
    // The fields default to u16
    pub u16, into Protocol, client_protocol, _ : 31, 16;
    pub u16, client_vendorid, _ : 15, 0;
}

bitfield! {
    pub struct InitializeResponseParameter(u32);
    impl Debug;
    // The fields default to u16
    pub u16, from into Protocol, negotiated_protocol, set_negotiated_protocol : 31, 16;
    pub u16, session_id, set_session_id : 15, 0;
}

impl InitializeResponseParameter {
    pub(crate) fn new(negotiated_protocol: Protocol, session_id: u16) -> Self {
        let mut x = InitializeResponseParameter(0);
        x.set_negotiated_protocol(negotiated_protocol);
        x.set_session_id(session_id);
        x
    }
}

bitfield! {
    pub struct InitializeResponseControl(u8);
    impl Debug;
    // The fields default to u16
    pub prefer_overlap, set_prefer_overlap : 0;
    pub encryption_mode, set_encryption_mode : 1;
    pub initial_encryption, set_initial_encryption : 2;
    pub u8, ivi_reserved, set_ivi_reserved : 5, 3;
    pub u8, vendor_specific, set_vendor_specific : 7, 6;
}

impl InitializeResponseControl {
    pub(crate) fn new(
        prefer_overlap: bool,
        encryption_mode: bool,
        initial_encryption: bool,
    ) -> Self {
        let mut x = InitializeResponseControl(0);
        x.set_prefer_overlap(prefer_overlap);
        x.set_encryption_mode(encryption_mode);
        x.set_initial_encryption(initial_encryption);
        x
    }
}

bitfield! {
    pub struct AsyncInitializeResponseParameter(u32);
    impl Debug;
    // The fields default to u16
    pub u16, server_vendor_id, set_server_vendor_id : 15, 0;
}

impl AsyncInitializeResponseParameter {
    pub(crate) fn new(server_vendor_id: u16) -> Self {
        let mut x = AsyncInitializeResponseParameter(0);
        x.set_server_vendor_id(server_vendor_id);
        x
    }
}

bitfield! {
    pub struct AsyncInitializeResponseControl(u8);
    impl Debug;
    // The fields default to u16
    pub secure_connection, set_secure_connection : 0;
    pub u8, ivi_reserved, set_ivi_reserved : 5, 2;
    pub u8, vendor_specific, set_vendor_specific : 7, 6;
}

impl AsyncInitializeResponseControl {
    pub(crate) fn new(secure_connection: bool) -> Self {
        let mut x = AsyncInitializeResponseControl(0);
        x.set_secure_connection(secure_connection);
        x
    }
}

bitfield! {
    pub struct RmtDeliveredControl(u8);
    impl Debug;
    // The fields default to u16
    pub rmt_delivered, set_rmt_delivered : 0;
}

impl Display for RmtDeliveredControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rmt: {}", self.rmt_delivered())
    }
}

bitfield! {
    pub struct FeatureBitmap(u8);
    impl Debug;
    // The fields default to u16
    pub overlapped, set_overlapped : 0;
    pub encryption, set_encryption : 1;
    pub initial_encryption, set_initial_encryption : 2;
}

impl FeatureBitmap {
    pub(crate) fn new(overlapped: bool, encryption: bool, initial_encryption: bool) -> Self {
        let mut s = FeatureBitmap(0);
        s.set_overlapped(overlapped);
        s.set_encryption(encryption);
        s.set_initial_encryption(initial_encryption);
        s
    }
}

impl Display for FeatureBitmap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "overlapped: {}, encryption: {}, initial_encryption: {}",
            self.overlapped(),
            self.encryption(),
            self.initial_encryption()
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum RequestLockControl {
    Failure = 0,
    Success = 1,
    Error = 2,
}

impl From<SharedLockError> for RequestLockControl {
    fn from(err: SharedLockError) -> Self {
        match err {
            SharedLockError::Timeout => RequestLockControl::Failure,
            _ => RequestLockControl::Error,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ReleaseLockControl {
    SuccessExclusive = 1,
    SuccessShared = 2,
    Error = 3,
}
