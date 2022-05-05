use std::{
    io::{self, Cursor, Error, ErrorKind, Read, Write},
    sync::Arc,
    time::Duration,
};

use async_listen::ListenExt;
use async_std::{
    net::{TcpListener, TcpStream, UdpSocket},
    task,
};
use async_trait::async_trait;
use byteorder::ReadBytesExt;

use crate::{
    client,
    common::{onc_rpc::record::write_record, xdr::portmapper::xdr::Mapping},
};

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
    /// Program not available
    ProgUnavail,
    /// Program version not available (se accepted version low-high in [MissmatchInfo])
    ProgMissmatch(MissmatchInfo),
    /// Procedure not available
    ProcUnavail,
    /// Arguments have too many or too few bytes to deserialize
    GarbageArgs,
    /// Internal error
    SystemErr,
    /// RPC version not supported
    RpcMissmatch(MissmatchInfo),
    /// Error during RPC authentication
    AuthError(AuthStat),
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

    async fn serve_tcp_stream(self: Arc<Self>, mut stream: TcpStream) -> io::Result<()>
    where
        Self: Sync,
    {
        loop {
            // Read message
            let fragment = read_record(&mut stream, 1024 * 1024).await?;

            let reply = self.clone().handle_message(fragment).await?;

            write_record(&mut stream, reply).await?;
            //stream.write_all(&reply).await?;
        }
    }

    async fn serve_tcp_stream_noreply(self: Arc<Self>, mut stream: TcpStream) -> io::Result<()>
    where
        Self: Sync,
    {
        loop {
            // Read message
            let fragment = read_record(&mut stream, 1024 * 1024).await?;

            let _reply = self.clone().handle_message(fragment).await?;
        }
    }

    async fn serve_udp_socket(self: Arc<Self>, socket: UdpSocket) -> io::Result<()>
    where
        Self: Sync,
    {
        loop {
            // Read message
            let mut buf = vec![0; 1500];
            let (n, peer) = socket.recv_from(&mut buf).await?;

            let reply = self.clone().handle_message(buf[..n].to_vec()).await?;

            socket.send_to(&reply, peer).await?;
        }
    }

    async fn handle_message(self: Arc<Self>, data_in: Vec<u8>) -> Result<Vec<u8>, Error>
    where
        Self: Sync,
    {
        let mut ret = Cursor::new(Vec::new());
        let mut data_in = Cursor::new(data_in);
        let mut msg = RpcMessage::default();
        msg.read_xdr(&mut data_in)?;
        log::trace!("-> {:?}", msg);

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
                        // Shouldn't be returned by call()
                        RpcError::RpcMissmatch(_) => unreachable!(),
                        RpcError::AuthError(_) => unreachable!(),
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
        log::trace!("<- {:?}", reply);

        let mut data_out = Cursor::new(Vec::new());
        reply.write_xdr(&mut data_out)?;
        data_out.write_all(&ret.into_inner()[..])?;

        Ok(data_out.into_inner())
    }

    async fn call(
        self: Arc<Self>,
        prog: u32,
        vers: u32,
        proc: u32,
        args: &mut Cursor<Vec<u8>>,
        ret: &mut Cursor<Vec<u8>>,
    ) -> Result<(), RpcError>
    where
        Self: Sync,
    {
        Err(RpcError::ProgUnavail)
    }
}

pub(crate) struct UdpRpcClient {
    xid: u32,
    prog: u32,
    vers: u32,
    socket: UdpSocket,
}

impl UdpRpcClient {
    pub(crate) fn new(prog: u32, vers: u32, socket: UdpSocket) -> Self {
        Self {
            xid: 0,
            prog,
            vers,
            socket,
        }
    }

    /// Call the null procedure of program/version
    pub(crate) async fn null(&mut self) -> Result<(), RpcError> {
        self.call(0, ()).await
    }

    /// Call the null procedure of program/version
    pub(crate) async fn null_no_reply(&mut self) -> Result<(), RpcError> {
        self.call_no_reply(0, ()).await
    }

    /// Call procedure `proc` with arguments of type `ARGS`. Returns `Ok(RET)` if successfull.
    pub(crate) async fn call<ARGS, RET>(&mut self, proc: u32, args: ARGS) -> Result<RET, RpcError>
    where
        ARGS: XdrEncode,
        RET: XdrDecode + Default,
    {
        self.xid += 1;

        let mut args_cursor = Cursor::new(Vec::new());

        // Send a call
        let msg = RpcMessage::call(self.xid, self.prog, self.vers, proc);
        msg.write_xdr(&mut args_cursor)?;
        args.write_xdr(&mut args_cursor)?;
        self.socket.send(&args_cursor.into_inner()).await?;

        // Read response
        let mut buf = vec![0; 1024];
        self.socket.recv(&mut buf).await?;
        let mut ret_cursor = Cursor::new(buf);

        // Deserialize and parse response
        let mut reply = RpcMessage::default();
        let mut ret: RET = Default::default();
        reply.read_xdr(&mut ret_cursor)?;
        match reply {
            RpcMessage {
                mtype:
                    MsgType::Reply(Replybody {
                        stat: ReplyStat::Accepted(accepted),
                    }),
                xid,
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
                        stat: ReplyStat::Denied(RejectedReply { stat }),
                    }),
                ..
            } => match stat {
                RejectStat::RpcMissmatch(m) => Err(RpcError::RpcMissmatch(m)),
                RejectStat::AuthError(err) => Err(RpcError::AuthError(err)),
            },
            RpcMessage {
                mtype: MsgType::Call(..),
                ..
            } => {
                todo!()
            }
        }
    }

    /// Call procedure `proc` with arguments of type `ARGS`. Returns `Ok(RET)` if successfull.
    pub(crate) async fn call_no_reply<ARGS>(
        &mut self,
        proc: u32,
        args: ARGS,
    ) -> Result<(), RpcError>
    where
        ARGS: XdrEncode,
    {
        self.xid += 1;

        let mut args_cursor = Cursor::new(Vec::new());

        // Send a call
        let msg = RpcMessage::call(self.xid, self.prog, self.vers, proc);
        msg.write_xdr(&mut args_cursor)?;
        args.write_xdr(&mut args_cursor)?;
        self.socket.send(&args_cursor.into_inner()).await?;

        Ok(())
    }
}

pub(crate) struct StreamRpcClient<IO> {
    xid: u32,
    prog: u32,
    vers: u32,
    io: IO,
}

impl<IO> StreamRpcClient<IO> {
    pub(crate) fn new(io: IO, prog: u32, vers: u32) -> Self {
        Self {
            xid: 0,
            io,
            prog,
            vers,
        }
    }
}

impl<IO> StreamRpcClient<IO>
where
    IO: AsyncRead + AsyncWrite + Unpin,
{
    /// Call the null procedure of program/version
    pub(crate) async fn null(&mut self) -> Result<(), RpcError> {
        self.call(0, ()).await
    }

    /// Call the null procedure of program/version
    pub(crate) async fn null_no_reply(&mut self) -> Result<(), RpcError> {
        self.call_no_reply(0, ()).await
    }

    /// Call procedure `proc` with arguments of type `ARGS`. Returns `Ok(RET)` if successfull.
    pub(crate) async fn call<ARGS, RET>(&mut self, proc: u32, args: ARGS) -> Result<RET, RpcError>
    where
        ARGS: XdrEncode,
        RET: XdrDecode + Default,
    {
        self.xid += 1;

        let mut args_cursor = Cursor::new(Vec::new());

        // Send a call
        let msg = RpcMessage::call(self.xid, self.prog, self.vers, proc);
        msg.write_xdr(&mut args_cursor)?;
        args.write_xdr(&mut args_cursor)?;
        write_record(&mut self.io, args_cursor.into_inner()).await?;

        // Read response
        let fragment = read_record(&mut self.io, 1024 * 1024).await?;
        let mut ret_cursor = Cursor::new(fragment);

        // Deserialize and parse response
        let mut reply = RpcMessage::default();
        let mut ret: RET = Default::default();
        reply.read_xdr(&mut ret_cursor)?;
        match reply {
            RpcMessage {
                mtype:
                    MsgType::Reply(Replybody {
                        stat: ReplyStat::Accepted(accepted),
                    }),
                xid,
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
                        stat: ReplyStat::Denied(RejectedReply { stat }),
                    }),
                ..
            } => match stat {
                RejectStat::RpcMissmatch(m) => Err(RpcError::RpcMissmatch(m)),
                RejectStat::AuthError(err) => Err(RpcError::AuthError(err)),
            },
            RpcMessage {
                mtype: MsgType::Call(..),
                ..
            } => {
                todo!()
            }
        }
    }

    /// Call procedure `proc` with arguments of type `ARGS`. Returns `Ok(RET)` if successfull.
    pub(crate) async fn call_no_reply<ARGS>(
        &mut self,
        proc: u32,
        args: ARGS,
    ) -> Result<(), RpcError>
    where
        ARGS: XdrEncode,
    {
        self.xid += 1;

        let mut args_cursor = Cursor::new(Vec::new());

        // Send a call
        let msg = RpcMessage::call(self.xid, self.prog, self.vers, proc);
        msg.write_xdr(&mut args_cursor)?;
        args.write_xdr(&mut args_cursor)?;
        write_record(&mut self.io, args_cursor.into_inner()).await?;

        Ok(())
    }
}

pub(crate) enum RpcClient {
    Udp(UdpRpcClient),
    Tcp(StreamRpcClient<TcpStream>),
}

impl RpcClient {
    /// Call the null procedure of program/version
    pub(crate) async fn null(&mut self) -> Result<(), RpcError> {
        self.call(0, ()).await
    }

    /// Call the null procedure of program/version
    pub(crate) async fn null_no_reply(&mut self) -> Result<(), RpcError> {
        self.call_no_reply(0, ()).await
    }

    /// Call procedure `proc` with arguments of type `ARGS`. Returns `Ok(RET)` if successfull.
    pub(crate) async fn call<ARGS, RET>(&mut self, proc: u32, args: ARGS) -> Result<RET, RpcError>
    where
        ARGS: XdrEncode,
        RET: XdrDecode + Default,
    {
        match self {
            RpcClient::Udp(client) => client.call(proc, args).await,
            RpcClient::Tcp(client) => client.call(proc, args).await,
        }
    }

    /// Call procedure `proc` with arguments of type `ARGS`. Returns `Ok(RET)` if successfull.
    pub(crate) async fn call_no_reply<ARGS>(
        &mut self,
        proc: u32,
        args: ARGS,
    ) -> Result<(), RpcError>
    where
        ARGS: XdrEncode,
    {
        match self {
            RpcClient::Udp(client) => client.call_no_reply(proc, args).await,
            RpcClient::Tcp(client) => client.call_no_reply(proc, args).await,
        }
    }
}
