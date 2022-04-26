use std::{
    io::{self, Cursor, Error, ErrorKind, Read, Write},
    time::Duration,
};

use async_listen::ListenExt;
use async_std::{net::TcpListener, task};
use async_trait::async_trait;
use byteorder::ReadBytesExt;

use crate::common::{onc_rpc::record::write_record, xdr::portmapper::xdr::Mapping};

use self::record::read_record;
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, StreamExt};

use super::xdr::{
    basic::{XdrDecode, XdrEncode},
    onc_rpc::xdr::{
        AcceptStat, AcceptedReply, AuthFlavour, AuthStat, MissmatchInfo, MsgType, OpaqueAuth,
        RejectStat, RejectedReply, ReplyStat, Replybody, RpcMessage,
    },
};

mod record;

/// An error which occured during an RPC call
///
#[derive(Debug)]
pub enum RpcError {
    ProgUnavail,
    ProgMissmatch(MissmatchInfo),

    ProcUnavail,
    /// Arguments have too many or too few bytes to deserialize
    GarbageArgs,
    /// Internal error
    SystemErr,
    /// (De-)serialiation error on RPC channel
    Io(Error),
}

impl From<Error> for RpcError {
    fn from(err: Error) -> Self {
        return Self::Io(err);
    }
}

#[async_trait]
pub(crate) trait RpcService {
    async fn nullproc(&mut self) -> Result<(), RpcError> {
        Ok(())
    }

    async fn call(
        &self,
        prog: u32,
        vers: u32,
        proc: u32,
        args: &mut Cursor<Vec<u8>>,
        ret: &mut Cursor<Vec<u8>>,
    ) -> Result<(), RpcError> {
        Err(RpcError::ProgUnavail)
    }

    async fn handle_message(&self, data_in: Vec<u8>) -> Result<Vec<u8>, Error> {
        let mut ret = Cursor::new(Vec::new());
        let mut data_in = Cursor::new(data_in);
        let mut msg = RpcMessage::default();
        msg.read_xdr(&mut data_in)?;
        println!("-> {:?}", msg);

        let xid = msg.xid;

        let stat = if let MsgType::Call(call) = msg.mtype {
            if call.rpc_vers != 2 {
                ReplyStat::rpc_vers_missmatch(2, 2)
            } else if call.cred.flavour != AuthFlavour::None {
                ReplyStat::auth_error(AuthStat::RejectedCred)
            } else if call.verf.flavour != AuthFlavour::None {
                ReplyStat::auth_error(AuthStat::RejectedVerf)
            } else {
                // OK call
                let res = self
                    .call(call.prog, call.vers, call.proc, &mut data_in, &mut ret)
                    .await;
                let stat = if let Err(err) = res {
                    match err {
                        RpcError::ProgUnavail => AcceptStat::ProgUnavail,
                        RpcError::ProgMissmatch(m) => AcceptStat::ProgMissmatch(m),
                        RpcError::ProcUnavail => AcceptStat::ProcUnavail,
                        RpcError::GarbageArgs => AcceptStat::GarbageArgs,
                        RpcError::SystemErr => AcceptStat::SystemErr,
                        RpcError::Io(err) => return Err(err),
                    }
                } else {
                    AcceptStat::Success
                };

                ReplyStat::Accepted(AcceptedReply {
                    verf: Default::default(),
                    stat,
                })
            }
        } else {
            return Err(ErrorKind::Unsupported.into());
        };
        let reply = RpcMessage {
            xid,
            mtype: MsgType::Reply(Replybody { stat }),
        };
        println!("<- {:?}", reply);

        let mut data_out = Cursor::new(Vec::new());
        reply.write_xdr(&mut data_out)?;
        data_out.write_all(&ret.into_inner()[..])?;

        Ok(data_out.into_inner())
    }
}

pub(crate) async fn serve_rpc<RD, WR, SERVICE>(
    mut reader: RD,
    mut writer: WR,
    service: SERVICE,
) -> io::Result<()>
where
    RD: AsyncRead + Unpin,
    WR: AsyncWrite + Unpin,
    SERVICE: RpcService + Sync,
{
    loop {
        // Read message
        let fragment = read_record(&mut reader, 1024 * 1024).await?;

        let reply = service.handle_message(fragment).await?;

        write_record(&mut writer, reply).await?;
    }
}

#[async_std::test]
async fn test_serve_rpc() {
    struct T;
    impl RpcService for T {}

    let listener = TcpListener::bind(("127.0.0.1", 5000)).await.unwrap();
    let mut incoming = listener
        .incoming()
        .log_warnings(|warn| log::warn!("Listening error: {}", warn))
        .handle_errors(Duration::from_millis(100))
        .backpressure(10);

    while let Some((token, stream)) = incoming.next().await {
        let peer = stream.peer_addr().unwrap();
        println!("Accepted from: {}", peer);

        task::spawn(async move {
            let (reader, writer) = stream.split();
            if let Err(err) = serve_rpc(reader, writer, T).await {
                log::warn!("Error processing client: {}", err)
            }
            drop(token);
        });
    }
}

pub(crate) struct RpcClient<IO> {
    xid: u32,
    io: IO
}

impl<IO> RpcClient<IO> {
    pub(crate) fn new(io: IO) -> Self {
        Self {
            xid: 0,
            io
        }
    }

    /// Call the null procedure of program/version
    pub(crate) async fn null(
        &mut self,
        prog: u32,
        vers: u32,
    ) -> Result<(), RpcError>
    where
        IO: AsyncRead + AsyncWrite + Unpin,
    {
        self.call(prog, vers, 0, ()).await
    }

    /// Call procedure `proc` with arguments of type `ARGS`. Returns `Ok(RET)` if successfull.
    pub(crate) async fn call<ARGS, RET>(
        &mut self,
        prog: u32,
        vers: u32,
        proc: u32,
        args: ARGS,
    ) -> Result<RET, RpcError>
    where
        IO: AsyncRead + AsyncWrite + Unpin,
        ARGS: XdrEncode,
        RET: XdrDecode + Default,
    {
        self.xid += 1;

        let mut args_cursor = Cursor::new(Vec::new());

        let msg = RpcMessage::call(self.xid, prog, vers, proc);
        msg.write_xdr(&mut args_cursor)?;
        args.write_xdr(&mut args_cursor)?;
        write_record(&mut self.io, args_cursor.into_inner()).await?;

        let fragment = read_record(&mut self.io, 1024 * 1024).await?;
        let mut ret_cursor = Cursor::new(fragment);

        let mut reply = RpcMessage::default();
        let mut ret: RET = Default::default();
        reply.read_xdr(&mut ret_cursor)?;
        match reply {
            RpcMessage {
                mtype:
                    MsgType::Reply(Replybody {
                        stat: ReplyStat::Accepted(accepted),
                    }),
                ..
            } => match accepted.stat {
                AcceptStat::Success => {
                    ret.read_xdr(&mut ret_cursor)?;
                    Ok(ret)
                }
                AcceptStat::ProgUnavail => Err(RpcError::ProgUnavail),
                AcceptStat::ProgMissmatch(m) => Err(RpcError::ProgMissmatch(m)),
                AcceptStat::ProcUnavail => Err(RpcError::ProcUnavail),
                AcceptStat::GarbageArgs => Err(RpcError::GarbageArgs),
                AcceptStat::SystemErr => Err(RpcError::SystemErr),
            },
            RpcMessage {
                mtype:
                    MsgType::Reply(Replybody {
                        stat: ReplyStat::Denied(rejected),
                    }),
                ..
            } => {
                todo!()
            }
            RpcMessage {
                mtype: MsgType::Call(..),
                ..
            } => {
                todo!()
            }
        }
    }
}

pub async fn call_rpc<RD, WR, ARGS, RET>(
    mut reader: RD,
    mut writer: WR,
    prog: u32,
    vers: u32,
    proc: u32,
    args: ARGS,
) -> Result<RET, RpcError>
where
    RD: AsyncRead + Unpin,
    WR: AsyncWrite + Unpin,
    ARGS: XdrEncode,
    RET: XdrDecode + Default,
{
    let mut args_cursor = Cursor::new(Vec::new());

    let msg = RpcMessage::call(1, prog, vers, proc);
    msg.write_xdr(&mut args_cursor)?;
    args.write_xdr(&mut args_cursor)?;
    write_record(&mut writer, args_cursor.into_inner()).await?;

    let fragment = read_record(&mut reader, 1024 * 1024).await?;
    let mut ret_cursor = Cursor::new(fragment);

    let mut reply = RpcMessage::default();
    let mut ret: RET = Default::default();
    reply.read_xdr(&mut ret_cursor)?;
    match reply {
        RpcMessage {
            mtype:
                MsgType::Reply(Replybody {
                    stat: ReplyStat::Accepted(accepted),
                }),
            ..
        } => match accepted.stat {
            AcceptStat::Success => {
                ret.read_xdr(&mut ret_cursor)?;
                Ok(ret)
            }
            AcceptStat::ProgUnavail => Err(RpcError::ProgUnavail),
            AcceptStat::ProgMissmatch(m) => Err(RpcError::ProgMissmatch(m)),
            AcceptStat::ProcUnavail => Err(RpcError::ProcUnavail),
            AcceptStat::GarbageArgs => Err(RpcError::GarbageArgs),
            AcceptStat::SystemErr => Err(RpcError::SystemErr),
        },
        RpcMessage {
            mtype:
                MsgType::Reply(Replybody {
                    stat: ReplyStat::Denied(rejected),
                }),
            ..
        } => {
            todo!()
        }
        RpcMessage {
            mtype: MsgType::Call(..),
            ..
        } => {
            todo!()
        }
    }
}

#[async_std::test]
async fn test_call_rpc() {
    let mut stream = async_std::net::TcpStream::connect("127.0.0.1:111")
        .await
        .unwrap();
    println!("Connected to {}", &stream.peer_addr().unwrap());
    let (mut reader, mut writer) = stream.split();

    let ret: i32 = call_rpc(reader, writer, 100000, 2, 3, Mapping::new(100000, 2, 6, 0))
        .await
        .unwrap();

    println!("Port = {ret}")
}
