use crate::rpc::onc_rpc::RpcDecoder;
use async_std::io::{BufReader};
use async_std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use async_std::stream::StreamExt;
use async_std::task;
use async_std::task::Poll;
use futures::channel::mpsc;
use std::io::{Read, Write};
use xdr_codec::{Pack, Unpack};

mod onc_rpc;
mod portmap;
mod vxi11;

enum Error {
    /// Could not register service with portmap
    FailedToRegister,
    /// Portmap already have this program, version, proto mapping
    AlreadyRegistered,
    ///
    ProgramUnavailable,
    ///
    ProgramMismatch { high: u32, low: u32 },
    ///
    ProcedureUnavailable,
    ///
    GarbageArgs,
    ///
    SystemError,
    /// IO Error
    Io(async_std::io::Error),
}

impl From<std::io::Error> for Error {
    fn from(io: std::io::Error) -> Self {
        Error::Io(io)
    }
}

type Result<T> = std::result::Result<T, Error>;

pub(crate) enum RpcProto {
    Tcp,
    Udp,
}

impl RpcProto {
    pub(crate) fn prot(&self) -> u32 {
        match self {
            RpcProto::Tcp => portmap::xdr::IPPROTO_TCP as u32,
            RpcProto::Udp => portmap::xdr::IPPROTO_UDP as u32,
        }
    }
}

struct RpcService {
    program: u32,
    vers: u32,
    port: u16,
}

impl RpcService {
    async fn accept(addr: impl ToSocketAddrs) -> Result<()> {
        let listener = TcpListener::bind(addr).await?;
        let mut incoming = listener.incoming();
        while let Some(stream) = incoming.next().await {
            let stream = stream?;
            println!("Accepting from: {}", stream.peer_addr()?);
            let _handle = task::spawn(Self::connection_loop(stream)); // 1
        }
        Ok(())
    }

    async fn connection_loop(stream: TcpStream) -> Result<()> {
        let reader = BufReader::new(&stream); // 2
        let mut lines = RpcDecoder::new(reader);

        while let Some(msg) = lines.next().await {
            let msg = msg?;
            log::debug!("<- {:?}", msg)
        }
        Ok(())
    }
}

trait RpcClient<Out, In>
where
    Out: Write,
    In: Read,
{
    const PROC_NULL: u32 = 0;

    /// Call procedure
    fn call<ARGS, RET>(&self, proc: u32, args: ARGS) -> Result<RET>
    where
        ARGS: Pack<Out>,
        RET: Unpack<In>;

    /// Call the null procedure
    fn call_null(&self) -> Result<()> {
        self.call(Self::PROC_NULL, ())
    }
}

struct RpcTcpClient {
    program: u32,
    vers: u32,
    addr: SocketAddr,
}

struct InnerTcpClient {

}

impl RpcTcpClient {

}

#[cfg(test)]
mod tests {
    use async_std::net::{SocketAddr, TcpStream, ToSocketAddrs};
    use async_std::{
        net::{IpAddr, TcpListener}, // 3
        prelude::*,                 // 1
        task,                       // 2
    };

    use super::{
        onc_rpc,
        portmap
    };

    use crate::rpc::onc_rpc::xdr::{rpc_msg, rejected_reply, accepted_reply, _reply_data};
    use std::io::Cursor;
    use std::net::Ipv4Addr;
    use byteorder::{NetworkEndian, WriteBytesExt};
    use byteorder::ByteOrder;
    use xdr_codec::{Pack, Unpack};
    use crate::rpc::RpcProto;
    use futures::{AsyncReadExt, StreamExt};
    use crate::rpc::onc_rpc::RpcDecoder;
    use async_std::io::BufReader;

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

    async fn accept_loop(addr: IpAddr) -> Result<()> {
        // 1

        let listener = TcpListener::bind(SocketAddr::new(addr, 4880)).await?; // 2
        let mut incoming = listener.incoming();
        while let Some(stream) = incoming.next().await { // 3
             // TODO
        }
        Ok(())
    }

    async fn register(addr: impl ToSocketAddrs) -> Result<()> {
        // 1
        let call_msg = onc_rpc::xdr::rpc_msg {
            xid: 0,
            body: onc_rpc::xdr::_body::CALL(onc_rpc::xdr::call_body {
                rpcvers: 2,
                prog: 100000,
                vers: 2,
                proc_: 2,
                cred: onc_rpc::xdr::opaque_auth::default(),
                verf: onc_rpc::xdr::opaque_auth::default()
            })
        };

        let mapping = portmap::xdr::mapping::new(0x0607AF, 1, RpcProto::Tcp, 1024);

        let mut stream = TcpStream::connect(addr).await?;
        println!("Connected to {}", &stream.peer_addr()?);

        let (reader, mut writer) = stream.split();

        let msg = "hello world";
        println!("<- {}", msg);
        let mut buf: Vec<u8> = vec![];
        let mut cur = Cursor::new(buf);
        cur.write_u32::<NetworkEndian>(0);
        call_msg.pack(&mut cur).unwrap();
        mapping.pack(&mut cur).unwrap();
        let len = cur.position();
        cur.set_position(0);
        cur.write_u32::<NetworkEndian>((len - 4) as u32 | 0x8000_0000);
        //let mut header = [0u8; 4];
        //NetworkEndian::write_u32(&mut header, cur.position() as u32 | 0x8000_0000);
        //writer.write_all(&header).await?;
        writer.write_all(cur.get_ref().as_slice()).await?;

        let reader = BufReader::new(reader);
        let mut rpc_reader = RpcDecoder::new(reader);
        if let Some(data) = rpc_reader.next().await {
            let mut cur = Cursor::new(data.unwrap());
            let (msg, s) = onc_rpc::xdr::rpc_msg::unpack(&mut cur).unwrap();
            match msg.body {
                onc_rpc::xdr::_body::REPLY(onc_rpc::xdr::reply_body::MSG_ACCEPTED(accepted_reply{ verf, reply_data })) => {
                    match reply_data {
                        _reply_data::SUCCESS(_) => {
                            let (port, s) = bool::unpack(&mut cur).unwrap();
                            println!("port = {}", port);
                        }
                        _reply_data::PROG_MISMATCH(info) => {}
                        _ => {}
                    }
                }
                onc_rpc::xdr::_body::REPLY(onc_rpc::xdr::reply_body::MSG_DENIED(body)) => {
                    match body {
                        rejected_reply::RPC_MISMATCH(info) => {
                            println!("missmatch version not {} - {}", info.low, info.high)
                        }
                        rejected_reply::AUTH_ERROR(stat) => {
                            println!("Authentication error {:?}", stat)
                        }
                    }
                }
                onc_rpc::xdr::_body::CALL(..) => {
                    println!("call?")
                }
            };


        }


        Ok(())
    }

    #[test]
    fn run() -> Result<()> {
        let fut = accept_loop(IpAddr::V4(Ipv4Addr::LOCALHOST));
        task::block_on(fut)
    }

    #[test]
    fn run2() -> Result<()> {
        let fut = register("127.0.0.1:111");
        task::block_on(fut)
    }
}
