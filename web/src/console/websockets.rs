use std::{collections::HashMap, sync::Arc, sync::Weak};

use async_std::prelude::*;
use byteorder::{ByteOrder, NetworkEndian};
use lxi_device::{
    lock::{LockHandle, Mutex, SharedLock, SpinMutex},
    Device,
};
use tide::{Middleware, Request, Result as TideResult};
use tide_websockets::{Message, WebSocket, WebSocketConnection};

enum LxiError {
    InvalidMessage,
    UnrecognizedMessageType,
    InvalidInitialization,
}
pub trait LxiDeviceProvider<DEV> {
    fn create_session(&self) -> Arc<Session<DEV>>;
}

pub struct Session<DEV> {
    id: u32,
    handle: LockHandle<DEV>,
}

impl<DEV> Session<DEV> {
    fn new(id: u32, handle: LockHandle<DEV>) -> Self {
        Self { id, handle }
    }
}

pub async fn handle<DEV, S>(request: Request<S>, mut stream: WebSocketConnection) -> TideResult<()>
where
    DEV: Device + Send + Sync + 'static,
    S: LxiDeviceProvider<DEV> + Send + Sync + Clone + 'static,
{
    let server = request.state().create_session();
    while let Some(Ok(msg)) = stream.next().await {
        match msg {
            Message::Binary(data) => {}
            // Reply to ping
            Message::Ping(data) => stream.send(Message::Pong(data)).await?,
            Message::Text(_) | Message::Pong(_) | Message::Close(_) => break,
        }
    }

    Ok(())
}
