use bitfield::bitfield;

pub mod errors;
pub mod messages;

/// Protocol version 1.0
pub const PROTOCOL_1_0: Protocol = Protocol(0x0100);
/// Protocol version 1.1
pub const PROTOCOL_1_1: Protocol = Protocol(0x0101);
/// Protocol version 2.0
pub const PROTOCOL_2_0: Protocol = Protocol(0x0200);
/// Highest protocol supported by this crate (2.0)
pub const SUPPORTED_PROTOCOL: Protocol = PROTOCOL_2_0;

/// Hislip can have two modes, synchronized and overlapped.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum Mode {
    Synchronized,
    Overlapped,
}

bitfield! {
    #[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
    pub struct Protocol(u16);
    impl Debug;
    // The fields default to u16
    pub u8, major, set_major : 15, 8;
    pub u8, minor, set_minor : 7, 0;
}

impl Protocol {
    pub fn as_parameter(&self, session_id: u16) -> u32 {
        ((self.0 as u32) << 16) | session_id as u32
    }
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Display as `major.minor`
        write!(f, "{}.{}", self.major(), self.minor())
    }
}

impl From<u16> for Protocol {
    fn from(x: u16) -> Self {
        Protocol(x)
    }
}

impl From<Protocol> for u16 {
    fn from(p: Protocol) -> Self {
        p.0
    }
}
