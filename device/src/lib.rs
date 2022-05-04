#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
use alloc::vec::Vec;

pub mod lock;
pub mod frontpanel;
pub mod util;
pub mod session;

pub trait Device {
    /// Execute a arbitrary command
    fn execute(&mut self, cmd: &Vec<u8>) -> Vec<u8>;

    // Return a current device status (STB) byte
    // Some flags (such as MAV) will be overriden.
    fn get_status(&mut self) -> u8;

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
