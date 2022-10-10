//! This crate contains an abstract device trait and common infrastructure used to create
//! a LXI device.
//!
//! The [Device] trait implements an abstract LXI device capable of receiving and excuting a command and some other common LXI tasks.
//!
//!
//!
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
use alloc::{boxed::Box, vec::Vec};
use trigger::Source;

#[cfg(feature = "experimental")]
pub mod frontpanel;

/// Instrument locking infrastructure
pub mod lock;
/// Internal device status/SRQ messaging channel
pub mod status;
/// Standard trigger sources
pub mod trigger;
/// Example/debugging devices
pub mod util;

#[derive(Debug)]
#[non_exhaustive]
pub enum DeviceError {
    NotSupported,
    IoTimeout,
    IoError,
}

pub trait Device {
    /// Execute a arbitrary command
    fn execute(&mut self, cmd: &[u8]) -> Option<Vec<u8>>;

    /// Return a current device status (STB) byte
    /// Some flags (such as MAV) will be ignored.
    ///
    /// Note: This should not reset any flags like a *STB? call would do.
    fn get_status(&mut self) -> Result<u8, DeviceError>;

    /// Send a trigger signal to device
    fn trigger(&mut self, source: Source) -> Result<(), DeviceError>;

    /// Send a clear signal to device
    fn clear(&mut self) -> Result<(), DeviceError>;

    /// Set remote/RMT state
    ///
    /// When in remote, frontpanel or any other local controls (except for 'local' button if any)
    /// should be ignored.
    /// If the device does not support a remote mode, it should return Err(())
    fn set_remote(&mut self, _remote: bool) -> Result<(), DeviceError>;

    /// Enable/disable lockout for 'local' button
    fn set_local_lockout(&mut self, _enable: bool) {
        // Do nothing
    }
}

// Blanket proxy implementation for boxed devices
impl<DEV: Device + ?Sized> Device for Box<DEV> {
    fn execute(&mut self, cmd: &[u8]) -> Option<Vec<u8>> {
        (**self).execute(cmd)
    }

    fn get_status(&mut self) -> Result<u8, DeviceError> {
        (**self).get_status()
    }

    fn trigger(&mut self, source: Source) -> Result<(), DeviceError> {
        (**self).trigger(source)
    }

    fn clear(&mut self) -> Result<(), DeviceError> {
        (**self).clear()
    }

    fn set_remote(&mut self, remote: bool) -> Result<(), DeviceError> {
        (**self).set_remote(remote)
    }

    fn set_local_lockout(&mut self, enable: bool) {
        (**self).set_local_lockout(enable)
    }
}
