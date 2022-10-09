use async_std::io;
use lxi_hislip::{client::Client, STANDARD_PORT};

use clap::Parser;

/// Simple program to greet a person
#[derive(clap::Parser)]
#[clap(author, version, about, long_about = None)]
struct Arguments {
    #[clap(short, long, default_value = "localhost")]
    ip: String,

    /// Number of times to greet
    #[clap(short, long, default_value_t = STANDARD_PORT)]
    port: u16,

    /// Sub-address
    #[clap(short, long, default_value = "hislip0")]
    subaddr: String,

    /// Command to run
    #[command(subcommand)]
    action: Action,
}

#[derive(clap::Subcommand)]
enum Action {
    /// Write command to instrument
    Write {
        /// Command to write
        command: String,
    },
    /// Read from instrument
    Read {
        /// Bytes to read
        n: u64,
    },
    /// Write command and read back from instrument
    Query {
        /// Command to write
        command: String,
        /// Bytes to read
        n: u64,
    },
    /// Get status byte
    Status,
    /// Clear instrument
    Clear,
}

#[async_std::main]
async fn main() -> Result<(), io::Error> {
    femme::with_level(log::LevelFilter::Debug);
    let args = Arguments::parse();

    let client = Client::open((args.ip.as_str(), args.port), 0x1234, "hislip0")
        .await
        .expect("Failed to connect");

    println!("{client:?}");

    Ok(())
}
