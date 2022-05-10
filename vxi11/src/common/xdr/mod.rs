//! XDR implementation and types for various protocols

pub mod basic;

pub mod prelude {
    pub use byteorder::{ReadBytesExt, WriteBytesExt, NetworkEndian};
    pub use super::basic::*;
}