//! XDR implementation and types for various protocols

pub mod basic;

pub mod prelude {
    pub use super::basic::*;
    pub use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
}
