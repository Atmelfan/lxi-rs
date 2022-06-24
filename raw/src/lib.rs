#![no_std]

extern crate alloc;

// Client
pub mod client;

// Server
pub mod server;

// Common definitions
pub mod common {}

pub const SOCKET_STANDARD_PORT: u16 = 5025;
