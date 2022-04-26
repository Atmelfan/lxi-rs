use std::io::{Cursor, Error, ErrorKind, Read, Write};

use async_trait::async_trait;

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
pub(crate) enum RpcError {
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
        &mut self,
        prog: u32,
        vers: u32,
        proc: u32,
        args: &mut Cursor<Vec<u8>>,
        ret: &mut Cursor<Vec<u8>>,
    ) -> Result<(), RpcError> {
        Err(RpcError::ProgUnavail)
    }

    async fn handle_message(
        &mut self,
        msg: RpcMessage,
        args: &mut Cursor<Vec<u8>>,
        ret: &mut Cursor<Vec<u8>>,
    ) -> Result<RpcMessage, Error> {
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
                let res = self.call(call.prog, call.vers, call.proc, args, ret).await;
                let stat = if let Err(err) = res {
                    match err {
                        RpcError::ProgUnavail => AcceptStat::ProgUnavail,
                        RpcError::ProgMissmatch(m) => AcceptStat::ProgMissmatch(m),
                        RpcError::ProcUnavail => AcceptStat::ProcUnavail,
                        RpcError::GarbageArgs => AcceptStat::GarbageArgs,
                        RpcError::SystemErr => AcceptStat::SystemErr,
                        RpcError::Io(err) => if err.kind() == ErrorKind::UnexpectedEof {
                            // EOF occurred during parsing of args
                            AcceptStat::GarbageArgs
                        } else {
                            // Something else
                            return Err(err)
                        }
                    }
                } else {
                    AcceptStat::Success
                };

                ReplyStat::Accepted(AcceptedReply { verf: Default::default(), stat })
            }
        } else {
            return Err(ErrorKind::Unsupported.into());
        };

        Ok(RpcMessage{xid,mtype:MsgType::Reply(Replybody { stat })})
    }
}
