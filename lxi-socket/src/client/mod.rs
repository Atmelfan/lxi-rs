use std::io::{Error, Result};

use async_std::io;
use async_std::io::{ReadExt, WriteExt};
use async_std::net::TcpStream;
use async_std::net::ToSocketAddrs;
use async_std::prelude::*;
use async_std::task;

struct SocketClient<IO> {
    stream: IO,
    write_term: u8,
    read_term: u8,
}

impl SocketClient<TcpStream> {
    async fn connect(addr: impl ToSocketAddrs) -> Result<Self> {
        let stream = TcpStream::connect("127.0.0.1:8080").await?;
        log::info!("Connected to {}", &stream.peer_addr().unwrap());

        Ok(Self {
            stream,
            read_term: b'\n',
            write_term: b'\n',
        })
    }
}

impl<IO> SocketClient<IO>{
    fn write_term(self, term_char: u8) -> Self {
        Self {
            write_term: term_char,
            ..self
        }
    }

    fn read_term(self, term_char: u8) -> Self {
        Self {
            read_term: term_char,
            ..self
        }
    }
}

impl<IO> SocketClient<IO>
where
    IO: async_std::io::Read + async_std::io::Write + Unpin,
{
    pub async fn write(&mut self, cmd: &[u8]) -> Result<()> {
        self.stream.write_all(cmd).await
    }

    pub async fn read(&mut self, bufs: &mut [u8]) -> Result<usize> {
        self.stream.read(bufs).await
    }
}

fn main() -> io::Result<()> {
    task::block_on(async {
        let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
        println!("Connected to {}", &stream.peer_addr()?);

        let msg = "hello world";
        println!("<- {}", msg);
        stream.write_all(msg.as_bytes()).await?;

        let mut buf = vec![0u8; 1024];
        let n = stream.read(&mut buf).await?;
        println!("-> {}\n", String::from_utf8_lossy(&buf[..n]));

        Ok(())
    })
}
