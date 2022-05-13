#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;

pub mod frontpanel;
pub mod lock;
pub mod session;
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
    fn execute(&mut self, cmd: &Vec<u8>) -> Vec<u8>;

    /// Return a current device status (STB) byte
    /// Some flags (such as MAV) will be ignored.
    fn get_status(&mut self) -> u8;

    /// Send a trigger signal to device
    fn trigger(&mut self) -> Result<(), DeviceError> {
        Err(DeviceError::NotSupported)
    }

    /// Set remote/RMT state
    ///
    /// When in remote, frontpanel or any other local controls (except for 'local' button if any)
    /// should be ignored.
    /// If the device does not support a remote mode, it should return Err(())
    fn set_remote(&mut self, _remote: bool) -> Result<(), ()> {
        // Do nothing
        Err(())
    }

    /// Enable/disable lockout for 'local' button
    fn set_local_lockout(&mut self, _enable: bool) {
        // Do nothing
    }
}
