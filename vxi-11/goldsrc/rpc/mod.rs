use std::io::Cursor;

use async_std::io::BufReader;
use async_std::net::{TcpListener, TcpStream, ToSocketAddrs};
use async_std::prelude::*;
use async_std::stream::StreamExt;
use async_std::task;
use byteorder::{NetworkEndian, WriteBytesExt};
use futures::io::{ReadHalf, WriteHalf};
use xdr_codec::{Pack, Unpack};

pub(crate) use client::*;
pub(crate) use service::*;

use crate::rpc::onc_rpc::RpcTcpDeframer;

// Basic RPC protocol
mod onc_rpc;

// RPC generic client
mod client;
// RPC generic service
mod service;
// RPC Protocols
mod portmap;
mod vxi11;

#[derive(Debug)]
pub(crate) enum Error {
    /// Could not register service with portmap
    FailedToRegister,
    /// Portmap already have this program, version, proto mapping
    AlreadyRegistered,
    ///
    ProgramUnavailable,
    ///
    ProgramMismatch { high: u32, low: u32 },
    ///
    RpcMismatch { high: u32, low: u32 },
    ///
    ProcedureUnavailable,
    ///
    GarbageArgs,
    ///
    SystemError,
    ///
    AuthenticationError,
    /// IO Error
    Io(async_std::io::Error),
}

impl From<std::io::Error> for Error {
    fn from(io: std::io::Error) -> Self {
        Error::Io(io)
    }
}

type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {}
