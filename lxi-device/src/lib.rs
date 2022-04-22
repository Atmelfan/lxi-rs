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
}
