//! 

pub(crate) mod xdr;

/// TCP port to use for portmapper/rpcbind
pub const PORTMAPPER_PORT: u16 = 111;

// Program constants
/// Portmapper program number
pub(crate) const PORTMAPPER_PROG: u32 = 100000;
// Portmapper program version
pub(crate) const PORTMAPPER_VERS: u32 = 2;

pub const PORTMAPPER_PROT_TCP: u32 = 6;
pub const PORTMAPPER_PROT_UDP: u32 = 17;


// Procedures
/// Null procedure
pub(crate) const PMAPPROC_NULL: u32 = 0;
/// Set procedure
pub(crate) const PMAPPROC_SET: u32 = 1;
/// Unset procedure
pub(crate) const PMAPPROC_UNSET: u32 = 2;
/// Getport procedure
pub(crate) const PMAPPROC_GETPORT: u32 = 3;
/// Dump procedure
pub(crate) const PMAPPROC_DUMP: u32 = 4;
/// Callit procedure
pub(crate) const PMAPPROC_CALLIT: u32 = 5;
