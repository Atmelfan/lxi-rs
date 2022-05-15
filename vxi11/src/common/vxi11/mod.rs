pub(crate) mod xdr;

/// VXI-11 async channel program number
pub const DEVICE_ASYNC: u32 = 0x0607B0;
/// VXI-11 async channel program version
pub const DEVICE_ASYNC_VERSION: u32 = 1;
// Async channel procedures
pub(crate) const DEVICE_ABORT: u32 = 1;

/// VXI-11 core channel program number
pub const DEVICE_CORE: u32 = 0x0607AF;
/// VXI-11 core channel program version
pub const DEVICE_CORE_VERSION: u32 = 1;
// Core channel procedures
pub(crate) const CREATE_LINK: u32 = 10;
pub(crate) const DEVICE_WRITE: u32 = 11;
pub(crate) const DEVICE_READ: u32 = 12;
pub(crate) const DEVICE_READSTB: u32 = 13;
pub(crate) const DEVICE_TRIGGER: u32 = 14;
pub(crate) const DEVICE_CLEAR: u32 = 15;
pub(crate) const DEVICE_REMOTE: u32 = 16;
pub(crate) const DEVICE_LOCAL: u32 = 17;
pub(crate) const DEVICE_LOCK: u32 = 18;
pub(crate) const DEVICE_UNLOCK: u32 = 19;
pub(crate) const DEVICE_ENABLE_SRQ: u32 = 20;
pub(crate) const DEVICE_DOCMD: u32 = 22;
pub(crate) const DESTROY_LINK: u32 = 23;
pub(crate) const CREATE_INTR_CHAN: u32 = 25;
pub(crate) const DESTROY_INTR_CHAN: u32 = 26;

/// VXI-11 interrupt channel program number
pub const DEVICE_INTR: u32 = 0x0607B1;
/// VXI-11 interrupt channel program version
pub const DEVICE_INTR_VERSION: u32 = 1;
// Interrupt channel procedures
pub(crate) const DEVICE_INTR_SRQ: u32 = 30;
