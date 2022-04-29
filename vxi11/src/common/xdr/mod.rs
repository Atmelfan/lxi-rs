//! XDR implementation and types for various protocols

pub mod basic;
pub mod onc_rpc;
pub mod portmapper;
pub mod vxi11;

pub mod prelude {
    pub use byteorder::{ReadBytesExt, WriteBytesExt, NetworkEndian};
    pub use super::basic::*;
}