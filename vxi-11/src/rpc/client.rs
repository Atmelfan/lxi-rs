use async_std::net::{TcpStream, ToSocketAddrs};
use async_std::prelude::*;
use async_trait::async_trait;
use futures::io::{ReadHalf, WriteHalf};
use std::io::Cursor;
use xdr_codec::{Pack, Unpack};

use crate::rpc::onc_rpc::write_tcp_message;
use crate::rpc::{self, onc_rpc, Error};

#[async_trait]
pub(crate) trait RpcClient {
    const PROC_NULL: u32 = 0;

    /// Call procedure
    async fn call<ARGS, RET>(&mut self, proc: u32, args: ARGS) -> rpc::Result<RET>
    where
        ARGS: Pack<Cursor<Vec<u8>>> + Send,
        RET: Unpack<Cursor<Vec<u8>>>;

    /// Call the null procedure
    async fn call_null(&mut self) -> rpc::Result<()> {
        self.call(Self::PROC_NULL, ()).await
    }
}

pub struct RpcTcpClient {
    program: u32,
    vers: u32,
    xid: u32,
    stream: Option<TcpStream>,
}

impl RpcTcpClient {
    pub(crate) fn new(program: u32, vers: u32) -> Self {
        RpcTcpClient {
            program,
            vers,
            xid: 0,
            stream: None,
        }
    }

    pub(crate) async fn connect(&mut self, addr: impl ToSocketAddrs) -> rpc::Result<()> {
        let stream = TcpStream::connect(addr).await?;
        let x = self.stream.replace(stream);
        drop(x); // Close connection by dropping it. Just making it obvious.
        Ok(())
    }

    pub(crate) async fn call_tcp<ARGS, RES>(&mut self, proc: u32, args: ARGS) -> rpc::Result<RES>
    where
        ARGS: xdr_codec::Pack<Cursor<Vec<u8>>>,
        RES: xdr_codec::Unpack<Cursor<Vec<u8>>>,
    {
        use crate::rpc::onc_rpc::xdr::{_reply_data, accepted_reply, rejected_reply};
        use crate::rpc::onc_rpc::RpcTcpDeframer;
        use async_std::io::BufReader;
        use byteorder::{NetworkEndian, WriteBytesExt};

        self.xid += 1;

        if let Some(stream) = self.stream.as_mut() {
            let call_msg = onc_rpc::xdr::rpc_msg {
                xid: self.xid,
                body: onc_rpc::xdr::_body::CALL(onc_rpc::xdr::call_body {
                    rpcvers: 2,
                    prog: self.program,
                    vers: self.vers,
                    proc_: proc,
                    cred: onc_rpc::xdr::opaque_auth::default(),
                    verf: onc_rpc::xdr::opaque_auth::default(),
                }),
            };
            write_tcp_message(stream, &call_msg, args).await?;

            let reader = BufReader::new(stream);
            let mut rpc_reader = RpcTcpDeframer::new(reader);
            if let Some(data) = rpc_reader.next().await {
                let mut cur = Cursor::new(data.unwrap());
                let (msg, _s) = onc_rpc::xdr::rpc_msg::unpack(&mut cur).unwrap();
                if msg.xid != self.xid {
                    return Err(Error::GarbageArgs);
                }
                match msg.body {
                    onc_rpc::xdr::_body::REPLY(onc_rpc::xdr::reply_body::MSG_ACCEPTED(
                        accepted_reply {
                            verf: _,
                            reply_data,
                        },
                    )) => match reply_data {
                        _reply_data::SUCCESS(_) => {
                            let (res, _) = RES::unpack(&mut cur).unwrap();
                            Ok(res)
                        }
                        _reply_data::PROG_MISMATCH(info) => Err(Error::ProgramMismatch {
                            high: info.high,
                            low: info.low,
                        }),
                        _reply_data::PROG_UNAVAIL => Err(Error::ProgramUnavailable),
                        _reply_data::PROC_UNAVAIL => Err(Error::ProcedureUnavailable),
                        _reply_data::GARBAGE_ARGS => Err(Error::GarbageArgs),
                        _reply_data::SYSTEM_ERR => Err(Error::SystemError),
                    },
                    onc_rpc::xdr::_body::REPLY(onc_rpc::xdr::reply_body::MSG_DENIED(body)) => {
                        match body {
                            rejected_reply::RPC_MISMATCH(info) => {
                                println!("missmatch version not {} - {}", info.low, info.high);
                                Err(Error::RpcMismatch {
                                    high: info.high,
                                    low: info.low,
                                })
                            }
                            rejected_reply::AUTH_ERROR(stat) => {
                                println!("Authentication error {:?}", stat);
                                Err(Error::AuthenticationError)
                            }
                        }
                    }
                    onc_rpc::xdr::_body::CALL(..) => {
                        println!("call?");
                        Err(Error::GarbageArgs)
                    }
                }
            } else {
                Err(Error::GarbageArgs)
            }
        } else {
            panic!()
        }
    }
}

#[async_trait]
impl RpcClient for RpcTcpClient {
    async fn call<ARGS, RET>(&mut self, proc: u32, args: ARGS) -> rpc::Result<RET>
    where
        ARGS: Pack<Cursor<Vec<u8>>> + Send,
        RET: Unpack<Cursor<Vec<u8>>>,
    {
        self.call_tcp(proc, args).await
    }
}

pub(crate) struct RpcUdpClient {}
