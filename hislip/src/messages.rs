use core::option::Option;
use core::result::Result;
use std::fmt::{Display, Formatter};

use bitfield::bitfield;

use byteorder::{BigEndian, ByteOrder, NetworkEndian};

use crate::errors::{Error, FatalErrorCode, NonFatalErrorCode};

#[derive(Debug, Copy, Clone)]
pub struct Header {
    pub message_type: MessageType,
    pub control_code: u8,
    pub message_parameter: u32,
    pub len: usize,
}

impl Header {
    pub const MESSAGE_HEADER_SIZE: usize = 16;

    pub fn from_buffer(x: &[u8]) -> Result<Header, Error> {
        if x.len() != Self::MESSAGE_HEADER_SIZE {
            Err(Error::Fatal(
                FatalErrorCode::PoorlyFormattedMessageHeader,
                b"Header too short",
            ))
        } else {
            let prolog = x.get(0..2).unwrap();
            if prolog != b"HS" {
                return Err(Error::Fatal(
                    FatalErrorCode::PoorlyFormattedMessageHeader,
                    b"Invalid prologue",
                ));
            }

            let len = BigEndian::read_u64(&x[8..16]) as usize;

            Ok(Header {
                message_type: MessageType::from_message_type(x[2]).ok_or(Error::NonFatal(
                    NonFatalErrorCode::UnrecognizedMessageType,
                    b"Unrecognized message type",
                ))?,
                control_code: x[3],
                message_parameter: BigEndian::read_u32(&x[4..8]),
                len,
            })
        }
    }

    pub fn pack_buffer(&self, x: &mut [u8]) {
        assert!(x.len() >= Self::MESSAGE_HEADER_SIZE);
        x[0] = b'H';
        x[1] = b'S';
        x[2] = self.message_type.get_message_type();
        x[3] = self.control_code;
        NetworkEndian::write_u32(&mut x[4..8], self.message_parameter);
        NetworkEndian::write_u64(&mut x[8..16], self.len as u64);
    }
}

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

    pub fn message(self, len: usize) -> Header {
        Header {
            message_type: self,
            control_code: 0,
            message_parameter: 0,
            len,
        }
    }

    pub fn message_params(self, len: usize, control_code: u8, message_parameter: u32) -> Header {
        Header {
            message_type: self,
            control_code,
            message_parameter,
            len,
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
    pub encryption_mandatory, set_encryption_mandatory : 1;
    pub secure_connection, set_secure_connection : 2;
    pub u8, ivi_reserved, set_ivi_reserved : 5, 3;
    pub u8, vendor_specific, set_vendor_specific : 7, 6;
}

impl InitializeResponseControl {
    pub(crate) fn new(
        prefer_overlap: bool,
        encryption_mandatory: bool,
        secure_connection: bool,
    ) -> Self {
        let mut x = InitializeResponseControl(0);
        x.set_prefer_overlap(prefer_overlap);
        x.set_encryption_mandatory(encryption_mandatory);
        x.set_secure_connection(secure_connection);
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
    pub struct FeatureBitmap(u8);
    impl Debug;
    // The fields default to u16
    pub overlapped, set_overlapped : 0;
    pub encryption, set_encryption : 1;
    pub initial_encryption, set_initial_encryption : 2;
}

bitfield! {
    #[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
    pub struct Protocol(u16);
    impl Debug;
    // The fields default to u16
    pub u8, major, set_major : 15, 8;
    pub u8, minor, set_minor : 7, 0;
}

impl Protocol {
    pub fn as_parameter(&self, session_id: u16) -> u32 {
        ((self.0 as u32) << 16) | session_id as u32
    }
}

impl Display for Protocol {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Display as `major.minor`
        write!(f, "{}.{}", self.major(), self.minor())
    }
}

impl From<u16> for Protocol {
    fn from(x: u16) -> Self {
        Protocol(x)
    }
}

impl From<Protocol> for u16 {
    fn from(p: Protocol) -> Self {
        p.0
    }
}
