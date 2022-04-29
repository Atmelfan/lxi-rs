use async_std::sync::Arc;
use async_std::net::TcpStream;
use futures::channel::mpsc;
use futures::StreamExt;

use crate::common::Protocol;
use crate::common::errors::Error;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SessionMode {
    Synchronized,
    Overlapped,
}

pub(crate) struct Session {
    pub(crate) sub_adress: String,
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
            id: session_id,
            max_message_size: 256,
            async_connected: false,
            async_encrypted: false,
        }
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
}
