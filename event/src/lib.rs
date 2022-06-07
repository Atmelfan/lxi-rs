use std::net::Ipv4Addr;

pub mod common;
pub mod server;

pub const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 23, 159);
pub const STANDARD_PORT: u16 = 5044;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
