pub mod common;
pub mod server;

/// Standard HiSLIP port number
pub const STANDARD_PORT: u16 = 4880;

/// Default device sub-adress.
/// Used if no other sub-adress was specified
pub const DEFAULT_DEVICE_SUBADRESS: &str = "hislip0";
