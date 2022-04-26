//! [RFC5531](https://datatracker.ietf.org/doc/html/rfc5531)
//!
//!
//!
//!

pub(crate) mod xdr {
    use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
    use std::io::{ErrorKind, Read, Result, Write};

    use crate::common::xdr::prelude::*;

    #[derive(Debug)]
    pub(crate) enum MsgType {
        Call(Callbody),
        Reply(Replybody),
    }

    impl Default for MsgType {
        fn default() -> Self {
            Self::Call(Default::default())
        }
    }

    impl XdrEncode for MsgType {
        fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
        where
            WR: Write,
        {
            match self {
                MsgType::Call(cb) => {
                    writer.write_u32::<NetworkEndian>(0)?;
                    cb.write_xdr(writer)
                }
                MsgType::Reply(rb) => {
                    writer.write_u32::<NetworkEndian>(1)?;
                    rb.write_xdr(writer)
                }
            }
        }
    }

    impl XdrDecode for MsgType {
        fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
        where
            RD: Read,
        {
            let discriminator = reader.read_u32::<NetworkEndian>()?;
            match discriminator {
                0 => {
                    let mut cb: Callbody = Default::default();
                    cb.read_xdr(reader)?;
                    *self = Self::Call(cb);
                }
                1 => {
                    let mut rb: Replybody = Default::default();
                    rb.read_xdr(reader)?;
                    *self = Self::Reply(rb);
                }
                _ => return Err(ErrorKind::Other.into()),
            };
            Ok(())
        }
    }

    #[derive(Debug)]
    pub(crate) enum ReplyStat {
        Accepted(AcceptedReply),
        Denied(RejectedReply),
    }

    impl ReplyStat {
        pub(crate) fn rpc_vers_missmatch(low: u32, high: u32) -> Self {
            Self::Denied(RejectedReply {
                stat: RejectStat::RpcMissmatch(MissmatchInfo { low, high }),
            })
        }

        pub(crate) fn auth_error(stat: AuthStat) -> Self {
            Self::Denied(RejectedReply {
                stat: RejectStat::AuthError(stat),
            })
        }
    }

    impl Default for ReplyStat {
        fn default() -> Self {
            Self::Accepted(Default::default())
        }
    }

    impl XdrEncode for ReplyStat {
        fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
        where
            WR: Write,
        {
            match self {
                ReplyStat::Accepted(areply) => {
                    writer.write_u32::<NetworkEndian>(0)?;
                    areply.write_xdr(writer)
                }
                ReplyStat::Denied(rreply) => {
                    writer.write_u32::<NetworkEndian>(1)?;
                    rreply.write_xdr(writer)
                }
            }
        }
    }

    impl XdrDecode for ReplyStat {
        fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
        where
            RD: Read,
        {
            let discriminant = reader.read_u32::<NetworkEndian>()?;
            *self = match discriminant {
                0 => {
                    let mut areply: AcceptedReply = Default::default();
                    areply.read_xdr(reader)?;
                    Self::Accepted(areply)
                }
                1 => {
                    let mut rreply: RejectedReply = Default::default();
                    rreply.read_xdr(reader)?;
                    Self::Denied(rreply)
                }
                _ => return Err(ErrorKind::Other.into()),
            };
            Ok(())
        }
    }

    #[derive(Debug)]
    pub(crate) enum AcceptStat {
        Success,
        ProgUnavail,
        ProgMissmatch(MissmatchInfo),
        ProcUnavail,
        GarbageArgs,
        SystemErr,
    }

    impl Default for AcceptStat {
        fn default() -> Self {
            Self::Success
        }
    }

    impl XdrEncode for AcceptStat {
        fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
        where
            WR: Write,
        {
            writer.write_u32::<NetworkEndian>(match *self {
                AcceptStat::Success => 0,
                AcceptStat::ProgUnavail => 1,
                AcceptStat::ProgMissmatch(_) => 2,
                AcceptStat::ProcUnavail => 3,
                AcceptStat::GarbageArgs => 4,
                AcceptStat::SystemErr => 5,
            })?;
            if let AcceptStat::ProgMissmatch(missmatch) = self {
                missmatch.write_xdr(writer)?;
            }
            Ok(())
        }
    }

    impl XdrDecode for AcceptStat {
        fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
        where
            RD: Read,
        {
            let discriminant = reader.read_u32::<NetworkEndian>()?;
            *self = match discriminant {
                0 => Self::Success,
                1 => Self::ProgUnavail,
                2 => {
                    let mut missmatch: MissmatchInfo = Default::default();
                    missmatch.read_xdr(reader)?;
                    Self::ProgMissmatch(missmatch)
                }
                3 => Self::ProcUnavail,
                4 => Self::GarbageArgs,
                5 => Self::SystemErr,
                _ => return Err(ErrorKind::Other.into()),
            };
            Ok(())
        }
    }

    #[derive(Debug)]
    pub(crate) enum RejectStat {
        RpcMissmatch(MissmatchInfo),
        AuthError(AuthStat),
    }

    impl Default for RejectStat {
        fn default() -> Self {
            Self::RpcMissmatch(Default::default())
        }
    }

    impl XdrEncode for RejectStat {
        fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
        where
            WR: Write,
        {
            match self {
                RejectStat::RpcMissmatch(missmatch) => {
                    writer.write_u32::<NetworkEndian>(0)?;
                    missmatch.write_xdr(writer)
                }
                RejectStat::AuthError(err) => {
                    writer.write_u32::<NetworkEndian>(1)?;
                    err.write_xdr(writer)
                }
            }
        }
    }

    impl XdrDecode for RejectStat {
        fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
        where
            RD: Read,
        {
            let discriminant = reader.read_u32::<NetworkEndian>()?;
            *self = match discriminant {
                0 => {
                    let mut missmatch: MissmatchInfo = Default::default();
                    missmatch.read_xdr(reader)?;
                    Self::RpcMissmatch(missmatch)
                }
                1 => {
                    let mut authstat: AuthStat = Default::default();
                    authstat.read_xdr(reader)?;
                    Self::AuthError(authstat)
                }
                _ => return Err(ErrorKind::Other.into()),
            };
            Ok(())
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub(crate) enum AuthStat {
        Ok,
        BadCred,
        RejectedCred,
        BadVerf,
        RejectedVerf,
        TooWeak,
        InvalidResp,
        Failed,
    }

    impl Default for AuthStat {
        fn default() -> Self {
            Self::Ok
        }
    }

    impl XdrEncode for AuthStat {
        fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
        where
            WR: Write,
        {
            writer.write_u32::<NetworkEndian>(match *self {
                AuthStat::Ok => 0,
                AuthStat::BadCred => 1,
                AuthStat::RejectedCred => 2,
                AuthStat::BadVerf => 3,
                AuthStat::RejectedVerf => 4,
                AuthStat::TooWeak => 5,
                AuthStat::InvalidResp => 6,
                AuthStat::Failed => 7,
            })
        }
    }

    impl XdrDecode for AuthStat {
        fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
        where
            RD: Read,
        {
            let discriminant = reader.read_u32::<NetworkEndian>()?;
            *self = match discriminant {
                0 => Self::Ok,
                1 => Self::BadCred,
                2 => Self::RejectedCred,
                3 => Self::BadVerf,
                4 => Self::RejectedVerf,
                5 => Self::TooWeak,
                6 => Self::InvalidResp,
                7 => Self::Failed,
                _ => return Err(ErrorKind::Other.into()),
            };
            Ok(())
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum AuthFlavour {
        None,
        Sys,
        Short,
    }

    impl Default for AuthFlavour {
        fn default() -> Self {
            Self::None
        }
    }

    impl XdrDecode for AuthFlavour {
        fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
        where
            RD: Read,
        {
            let discrimant = reader.read_u32::<NetworkEndian>()?;
            *self = match discrimant {
                0 => Self::None,
                1 => Self::Sys,
                2 => Self::Short,
                _ => return Err(ErrorKind::Other.into()),
            };
            Ok(())
        }
    }

    impl XdrEncode for AuthFlavour {
        fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
        where
            WR: Write,
        {
            writer.write_u32::<NetworkEndian>(match self {
                AuthFlavour::None => 0,
                AuthFlavour::Sys => 1,
                AuthFlavour::Short => 2,
            })
        }
    }

    #[derive(Debug, Default)]
    pub(crate) struct OpaqueAuth {
        pub flavour: AuthFlavour,
        pub body: Vec<u8>,
    }

    impl XdrEncode for OpaqueAuth {
        fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
        where
            WR: Write,
        {
            self.flavour.write_xdr(writer)?;
            self.body.write_xdr(writer)
        }
    }
    impl XdrDecode for OpaqueAuth {
        fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
        where
            RD: Read,
        {
            self.flavour.read_xdr(reader)?;
            self.body.read_xdr(reader)
        }
    }

    #[derive(Debug, Default)]
    pub(crate) struct Callbody {
        pub rpc_vers: u32,
        pub prog: u32,
        pub vers: u32,
        pub proc: u32,
        pub cred: OpaqueAuth,
        pub verf: OpaqueAuth, /* Args follows */
    }

    impl XdrEncode for Callbody {
        fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
        where
            WR: Write,
        {
            self.rpc_vers.write_xdr(writer)?;
            self.prog.write_xdr(writer)?;
            self.vers.write_xdr(writer)?;
            self.proc.write_xdr(writer)?;
            self.cred.write_xdr(writer)?;
            self.verf.write_xdr(writer)
        }
    }

    impl XdrDecode for Callbody {
        fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
        where
            RD: Read,
        {
            self.rpc_vers.read_xdr(reader)?;
            self.prog.read_xdr(reader)?;
            self.vers.read_xdr(reader)?;
            self.proc.read_xdr(reader)?;
            self.cred.read_xdr(reader)?;
            self.verf.read_xdr(reader)
        }
    }

    #[derive(Debug, Default, PartialEq, PartialOrd)]
    pub struct MissmatchInfo {
        pub low: u32,
        pub high: u32,
    }

    impl XdrEncode for MissmatchInfo {
        fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
        where
            WR: Write,
        {
            self.low.write_xdr(writer)?;
            self.high.write_xdr(writer)
        }
    }

    impl XdrDecode for MissmatchInfo {
        fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
        where
            RD: Read,
        {
            self.low.read_xdr(reader)?;
            self.high.read_xdr(reader)
        }
    }

    #[derive(Debug, Default)]
    pub(crate) struct Replybody {
        pub stat: ReplyStat,
    }

    impl XdrEncode for Replybody {
        fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
        where
            WR: Write,
        {
            self.stat.write_xdr(writer)
        }
    }

    impl XdrDecode for Replybody {
        fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
        where
            RD: Read,
        {
            self.stat.read_xdr(reader)
        }
    }

    #[derive(Debug, Default)]
    pub(crate) struct AcceptedReply {
        pub verf: OpaqueAuth,
        pub stat: AcceptStat,
    }

    impl XdrEncode for AcceptedReply {
        fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
        where
            WR: Write,
        {
            self.verf.write_xdr(writer)?;
            self.stat.write_xdr(writer)
        }
    }

    impl XdrDecode for AcceptedReply {
        fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
        where
            RD: Read,
        {
            self.verf.read_xdr(reader)?;
            self.stat.read_xdr(reader)
        }
    }

    #[derive(Debug, Default)]
    pub(crate) struct RejectedReply {
        pub stat: RejectStat,
    }

    impl XdrEncode for RejectedReply {
        fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
        where
            WR: Write,
        {
            self.stat.write_xdr(writer)
        }
    }

    impl XdrDecode for RejectedReply {
        fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
        where
            RD: Read,
        {
            self.stat.read_xdr(reader)
        }
    }

    /// A Rpc call or reply
    #[derive(Debug, Default)]
    pub(crate) struct RpcMessage {
        pub xid: u32,
        pub mtype: MsgType,
    }

    impl XdrEncode for RpcMessage {
        fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
        where
            WR: Write,
        {
            self.xid.write_xdr(writer)?;
            self.mtype.write_xdr(writer)
        }
    }

    impl XdrDecode for RpcMessage {
        fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
        where
            RD: Read,
        {
            self.xid.read_xdr(reader)?;
            self.mtype.read_xdr(reader)
        }
    }

    impl RpcMessage {


        pub(crate) fn call(xid: u32, prog: u32, vers: u32, proc: u32) -> Self  {
            Self {
                xid,
                mtype: MsgType::Call(Callbody { rpc_vers: 2, prog, vers, proc, cred: Default::default(), verf: Default::default() })
            }
        }
    }
}
