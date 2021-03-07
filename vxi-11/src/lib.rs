//! VXI11
//!
//!

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

mod protocol;
mod server;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
