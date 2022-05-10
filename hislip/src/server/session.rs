use async_std::net::TcpStream;
use async_std::sync::Arc;
use futures::channel::mpsc;
use futures::StreamExt;
use lxi_device::lock::LockHandle;

use crate::common::errors::Error;
use crate::common::Protocol;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SessionMode {
    Synchronized,
    Overlapped,
}

pub(crate) struct Session<DEV> {
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
    pub(crate) fn new(session_id: u16, protocol: Protocol, handle: LockHandle<DEV>) -> Self {
        Self {
            protocol,
            mode: SessionMode::Synchronized,
            id: session_id,
            max_message_size: 256,
            async_connected: false,
            async_encrypted: false,
            handle,
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
}

impl<DEV> Drop for Session<DEV> {
    fn drop(&mut self) {
        self.close()
    }
}
