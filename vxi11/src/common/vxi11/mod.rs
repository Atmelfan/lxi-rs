#![allow(non_upper_case_globals)]

pub(crate) mod xdr;

/// VXI-11 async channel program number
pub const DEVICE_ASYNC: u32 = 0x0607B0;
/// VXI-11 async channel program version
pub const DEVICE_ASYNC_VERSION: u32 = 1;
// Async channel procedures
pub(crate) const device_abort: u32 = 1;

/// VXI-11 core channel program number
pub const DEVICE_CORE: u32 = 0x0607AF;
/// VXI-11 core channel program version
pub const DEVICE_CORE_VERSION: u32 = 1;
// Core channel procedures
pub(crate) const create_link: u32 = 10;
pub(crate) const device_write: u32 = 11;
pub(crate) const device_read: u32 = 12;
pub(crate) const device_readstb: u32 = 13;
pub(crate) const device_trigger: u32 = 14;
pub(crate) const device_clear: u32 = 15;
pub(crate) const device_remote: u32 = 16;
pub(crate) const device_local: u32 = 17;
pub(crate) const device_lock: u32 = 18;
pub(crate) const device_unlock: u32 = 19;
pub(crate) const device_enable_srq: u32 = 20;
pub(crate) const device_docmd: u32 = 22;
pub(crate) const destroy_link: u32 = 23;
pub(crate) const create_intr_chan: u32 = 25;
pub(crate) const destroy_intr_chan: u32 = 26;

/// VXI-11 interrupt channel program number
pub const DEVICE_INTR: u32 = 0x0607B1;
/// VXI-11 interrupt channel program version
pub const DEVICE_INTR_VERSION: u32 = 1;
// Interrupt channel procedures
pub(crate) const device_intr_srq: u32 = 30;
