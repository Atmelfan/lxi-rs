use common::Protocol;

pub mod common;
pub mod server;

pub const PROTOCOL_1_1: Protocol = Protocol(257);
pub const PROTOCOL_2_0: Protocol = Protocol(512);

pub const STANDARD_PORT: u16 = 4880;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
