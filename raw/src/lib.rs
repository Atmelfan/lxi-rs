pub mod server;

pub mod common {}

/// Standard port for raw SCPI socket communication
pub const SOCKET_STANDARD_PORT: u16 = 5025;
/// Our standard port for secure raw communication.
/// **This is not a LXI standard port, just ours!**
pub const TLS_PORT: u16 = 6025;

