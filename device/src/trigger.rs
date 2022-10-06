

/// Source of a trigger signal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Trigger from a command or network server (for example VXI-11 trigger command).
    Bus,

    /// Trigger sent by INITiate
    Immediate,

    /// Internal trigger from example a timer
    Internal,

    /// Trigger from an external input
    External,

    /// Trigger from an LXI wired bus
    Lxi0,
    Lxi1,
    Lxi2,
    Lxi3,
    Lxi4,
    Lxi5,
    Lxi6,
    Lxi7,

    /// Triggers from a LXI network event
    Lan0,
    Lan1,
    Lan2,
    Lan3,
    Lan4,
    Lan5,
    Lan6,
    Lan7,
}