use std::io::{Cursor, Write};

use async_std::net::{TcpStream, ToSocketAddrs};
use byteorder::{ByteOrder, NetworkEndian};
use futures::{AsyncRead, AsyncWrite, AsyncWriteExt};

use crate::common::{
    errors::{Error, FatalErrorCode, NonFatalErrorCode},
    messages::{
        send_fatal, AsyncInitializeResponseControl, AsyncInitializeResponseParameter,
        InitializeParameter, InitializeResponseControl, InitializeResponseParameter, Message,
        MessageType,
    },
    Mode, Protocol, PROTOCOL_2_0, SUPPORTED_PROTOCOL,
};

#[derive(Debug)]
pub enum ClientError {
    Io(std::io::Error),
    Hislip(Error),
}

impl From<std::io::Error> for ClientError {
    fn from(io: std::io::Error) -> Self {
        Self::Io(io)
    }
}

impl From<Error> for ClientError {
    fn from(io: Error) -> Self {
        Self::Hislip(io)
    }
}

#[derive(Debug)]
pub struct Client {
    session_id: u16,
    overlap: bool,
    protocol: Protocol,
    server_vendorid: u16,
    client_vendorid: u16,

    server_maxlen: u64,
    client_maxlen: u64,

    server_messageid: u32,
    client_messageid: u32,

    synch: TcpStream,
    asynch: TcpStream,
}

impl Client {
    pub async fn open(
        addrs: impl ToSocketAddrs + Clone,
        client_vendorid: u16,
        sub_address: &str,
    ) -> Result<Self, ClientError> {
        // Create sync channel
        let mut synch = TcpStream::connect(addrs.clone()).await?;
        let init_param = InitializeParameter::new(SUPPORTED_PROTOCOL, client_vendorid);
        MessageType::Initialize
            .message_params(0, init_param.0)
            .with_payload(sub_address.as_bytes().to_vec())
            .write_to(&mut synch)
            .await?;
        let (overlap, protocol, session_id) = if let Message {
            message_type: MessageType::InitializeResponse,
            control_code,
            message_parameter,
            ..
        } = Message::read_from(&mut synch, 1000).await??
        {
            let control = InitializeResponseControl(control_code);
            let parameter = InitializeResponseParameter(message_parameter);

            let version = parameter.negotiated_protocol();

            // Check if server mandates encryption
            if version >= PROTOCOL_2_0 && control.encryption_mode() {
                return Err(Error::Fatal(
                    FatalErrorCode::InvalidInitialization,
                    "Server mandates secure connection, which is not supported".to_string(),
                )
                .into());
            }

            (
                control.prefer_overlap(),
                parameter.negotiated_protocol(),
                parameter.session_id(),
            )
        } else {
            return Err(Error::Fatal(
                FatalErrorCode::InvalidInitialization,
                "Unexpected response".to_string(),
            )
            .into());
        };

        // Create async channel
        let mut asynch = TcpStream::connect(addrs).await?;
        MessageType::AsyncInitialize
            .message_params(0, session_id as u32)
            .write_to(&mut asynch)
            .await?;
        let (_secure_connection, server_vendorid) = if let Message {
            message_type: MessageType::AsyncInitializeResponse,
            control_code,
            message_parameter,
            ..
        } =
            Message::read_from(&mut asynch, 1000).await??
        {
            let control = AsyncInitializeResponseControl(control_code);
            let parameter = AsyncInitializeResponseParameter(message_parameter);

            (control.secure_connection(), parameter.server_vendor_id())
        } else {
            return Err(Error::Fatal(
                FatalErrorCode::InvalidInitialization,
                "Unexpected response".to_string(),
            )
            .into());
        };

        // Negotiate buffer sizes
        let client_maxlen: u64 = 1024;
        let mut buf = vec![0; 8];
        NetworkEndian::write_u64(buf.as_mut_slice(), client_maxlen);
        MessageType::AsyncMaximumMessageSize
            .message_params(0, 0)
            .with_payload(buf)
            .write_to(&mut asynch)
            .await?;
        let server_maxlen = if let Message {
            message_type: MessageType::AsyncMaximumMessageSizeResponse,
            payload,
            ..
        } = Message::read_from(&mut asynch, 1000).await??
        {
            if payload.len() != 8 {
                return Err(Error::Fatal(
                    FatalErrorCode::UnidentifiedError,
                    "Unexpected payload size".to_string(),
                )
                .into());
            }
            NetworkEndian::read_u64(payload.as_slice())
        } else {
            return Err(Error::Fatal(
                FatalErrorCode::UnidentifiedError,
                "Unexpected response".to_string(),
            )
            .into());
        };

        Ok(Self {
            session_id,
            overlap,
            protocol,
            server_vendorid,
            client_vendorid,
            synch,
            asynch,
            server_maxlen,
            client_maxlen,
            server_messageid: 0xffffff00,
            client_messageid: 0xffffff00,
        })
    }

    pub async fn write(&mut self, data: &[u8], end: bool) -> Result<(), ClientError> {
        let chunks = data.chunks(self.server_maxlen.min(usize::MAX as u64) as usize);
        let n = chunks.len();
        for (i, chunk) in chunks.enumerate() {
            let t = if i == n - 1 && end {
                MessageType::DataEnd
            } else {
                MessageType::Data
            };
            t.message_params(0, self.client_messageid)
                .with_payload(chunk.to_vec())
                .write_to(&mut self.synch)
                .await?;

            self.client_messageid = self.client_messageid.wrapping_add(2);
        }
        Ok(())
    }

    pub async fn read(&mut self, data: &mut [u8]) -> Result<(), ClientError> {
        let mut vec = Vec::new();
        loop {
            match &mut Message::read_from(&mut self.synch, self.client_maxlen).await?? {
                Message {
                    message_type: MessageType::Data | MessageType::DataEnd,
                    payload,
                    ..
                } => {
                    vec.append(payload);
                }
                Message {
                    message_type: MessageType::Interrupted,
                    ..
                } => {
                    vec.clear();
                }
                _ => {
                    return Err(Error::Fatal(
                        FatalErrorCode::UnidentifiedError,
                        "Unexpected response".to_string(),
                    )
                    .into());
                }
            }
        }
    }

    /// Explicit close.
    /// Just dropping it also works
    pub async fn close(mut self) -> Result<(), ClientError> {
        self.synch.close().await?;
        self.asynch.close().await?;
        Ok(())
    }
}
