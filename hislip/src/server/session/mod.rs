use std::sync::Weak;

use async_std::{channel::{self, Receiver, Sender}};
use lxi_device::{Device, lock::{Mutex, SpinMutex, LockHandle}};

use super::ServerConfig;
use crate::common::Protocol;

pub(crate) mod asynchronous;
pub(crate) mod synchronous;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SessionMode {
    Synchronized,
    Overlapped,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub(crate) enum SessionState {
    Handshake,
    Normal,
}

pub(crate) struct SharedSession {
    /// Negotiated rpc
    protocol: Protocol,

    /// Current state of session
    state: SessionState,

    /// Negotiated session mode
    mode: SessionMode,

    /// Client max message size
    max_message_size: u64,

    /// Should enable remote automatically
    enable_remote: bool,

    clear: (Sender<()>, Receiver<()>),

    read_message_id: u32,
    sent_message_id: u32,
}

impl SharedSession {
    pub(crate) fn new(protocol: Protocol) -> Self {
        Self {
            protocol,
            state: SessionState::Handshake,
            mode: SessionMode::Overlapped,
            max_message_size: 256,
            clear: channel::bounded(1),
            read_message_id: 0,
            enable_remote: true,
            sent_message_id: 0,
        }
    }

    /// Get the session's state.
    #[must_use]
    pub(crate) fn state(&self) -> SessionState {
        self.state
    }

    #[must_use]
    pub(crate) fn is_initialized(&self) -> bool {
        // !matches!(self.state, SessionState::Handshake) // Just looks weird
        match self.state {
            SessionState::Handshake => false,
            _ => false,
        }
    }

    /// Get the session's protocol.
    #[must_use]
    pub(crate) fn protocol(&self) -> Protocol {
        self.protocol
    }

    /// Set the session's state.
    pub(crate) fn set_state(&mut self, state: SessionState) {
        self.state = state;
    }

    pub(crate) fn get_clear_receiver(&self) -> Receiver<()> {
        self.clear.1.clone()
    }

    pub(crate) fn get_clear_sender(&self) -> Sender<()> {
        self.clear.0.clone()
    }
}

/// A handle to a created active season
#[derive(Clone)]
pub(crate) struct SessionHandle<DEV>
where
    DEV: Device,
{
    _id: u16,
    pub shared: Weak<Mutex<SharedSession>>,
    pub device: Weak<SpinMutex<LockHandle<DEV>>>,
}

impl<DEV> SessionHandle<DEV>
where
    DEV: Device,
{
    pub(crate) fn new(
        id: u16,
        session: Weak<Mutex<SharedSession>>,
        handle: Weak<SpinMutex<LockHandle<DEV>>>,
    ) -> Self {
        Self {
            _id: id,
            shared: session,
            device: handle,
        }
    }

    /// Return false if the assosciated object have been closed
    pub(crate) fn active(&self) -> bool {
        self.shared.strong_count() > 0 && self.device.strong_count() > 0
    }
}