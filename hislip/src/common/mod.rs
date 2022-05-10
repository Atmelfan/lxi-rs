use bitfield::bitfield;

pub mod errors;
pub mod messages;

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
