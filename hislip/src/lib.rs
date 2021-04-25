use protocol::messages::Protocol;

pub mod protocol;
pub mod server;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub const PROTOCOL_1_1: Protocol = Protocol(257);
pub const PROTOCOL_2_0: Protocol = Protocol(512);

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
