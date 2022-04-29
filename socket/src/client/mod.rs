use std::io::Result;

use async_std::{
    io::{ReadExt, WriteExt},
    net::{TcpStream, ToSocketAddrs},
};

pub struct SocketClient<IO> {
    stream: IO,
    write_term: u8,
    read_term: u8,
}

impl SocketClient<TcpStream> {
    pub async fn connect(_addr: impl ToSocketAddrs) -> Result<Self> {
        let stream = TcpStream::connect("127.0.0.1:8080").await?;
        log::info!("Connected to {}", &stream.peer_addr().unwrap());

        Ok(Self {
            stream,
            read_term: b'\n',
            write_term: b'\n',
        })
    }
}

impl<IO> SocketClient<IO> {
    pub fn write_term(self, term_char: u8) -> Self {
        Self {
            write_term: term_char,
            ..self
        }
    }

    pub fn read_term(self, term_char: u8) -> Self {
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
