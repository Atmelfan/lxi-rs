use async_std::io;
use async_std::net::TcpStream;
use async_std::prelude::*;
use async_std::task;

fn main() -> io::Result<()> {
    env_logger::init();
    
    task::block_on(async {
        let mut stream = TcpStream::connect("127.0.0.1:5025").await?;
        println!("Connected to {}", &stream.peer_addr()?);

        let msg = "hello world\n";
        println!("<- {}", msg);
        stream.write_all(msg.as_bytes()).await?;

        let mut buf = vec![0u8; 1024];
        let n = stream.read(&mut buf).await?;
        println!("-> {}\n", String::from_utf8_lossy(&buf[..n]));

        Ok(())
    })
}
