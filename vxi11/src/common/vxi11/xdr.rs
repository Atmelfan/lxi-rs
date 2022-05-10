use std::io::{Read, Result, Write};

use crate::common::xdr::prelude::*;

#[derive(Debug, Clone, Copy)]
pub(crate) enum DeviceAddrFamily {
    Tcp,
    Udp,
    _Invalid,
}

impl Default for DeviceAddrFamily {
    fn default() -> Self {
        DeviceAddrFamily::Tcp
    }
}

impl XdrEncode for DeviceAddrFamily {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
    where
        WR: Write,
    {
        writer.write_u32::<NetworkEndian>(match self {
            DeviceAddrFamily::Tcp => 1,
            DeviceAddrFamily::Udp => 2,
            _ => panic!(),
        })
    }
}

impl XdrDecode for DeviceAddrFamily {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
    where
        RD: Read,
    {
        let discriminant = reader.read_u32::<NetworkEndian>()?;
        *self = match discriminant {
            1 => DeviceAddrFamily::Tcp,
            2 => DeviceAddrFamily::Udp,
            _ => DeviceAddrFamily::_Invalid,
        };
        Ok(())
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct DeviceLink(pub u32);

impl From<u32> for DeviceLink {
    fn from(x: u32) -> Self {
        DeviceLink(x)
    }
}

impl XdrEncode for DeviceLink {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
    where
        WR: Write,
    {
        self.0.write_xdr(writer)
    }
}

impl XdrDecode for DeviceLink {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
    where
        RD: Read,
    {
        self.0.read_xdr(reader)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct DeviceFlags(pub u32);

impl DeviceFlags {
    pub(crate) fn is_waitlock(&self) -> bool {
        (self.0 & 0x01) != 0
    }

    pub(crate) fn is_end(&self) -> bool {
        (self.0 & 0x08) != 0
    }

    pub(crate) fn is_termcharset(&self) -> bool {
        (self.0 & 0x80) != 0
    }
}

impl XdrEncode for DeviceFlags {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
    where
        WR: Write,
    {
        self.0.write_xdr(writer)
    }
}

impl XdrDecode for DeviceFlags {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
    where
        RD: Read,
    {
        self.0.read_xdr(reader)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) enum DeviceErrorCode {
    NoError,
    SyntaxError,
    DeviceNotAccessible,
    InvalidLinkIdentifier,
    ParameterError,
    ChannelNotEstablished,
    OperationNotSupported,
    OutOfResources,
    DeviceLockedByAnotherLink,
    NoLockHeldByThisLink,
    IoTimeout,
    IoError,
    InvalidAddress,
    Abort,
    ChannelAlreadyEstablished,

    /// Used for reserved/unknown error codes
    _Reserved(u32),
}

impl Default for DeviceErrorCode {
    fn default() -> Self {
        DeviceErrorCode::NoError
    }
}

impl XdrEncode for DeviceErrorCode {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
    where
        WR: Write,
    {
        writer.write_u32::<NetworkEndian>(match self {
            DeviceErrorCode::NoError => 0,
            DeviceErrorCode::SyntaxError => 1,
            DeviceErrorCode::DeviceNotAccessible => 3,
            DeviceErrorCode::InvalidLinkIdentifier => 4,
            DeviceErrorCode::ParameterError => 5,
            DeviceErrorCode::ChannelNotEstablished => 6,
            DeviceErrorCode::OperationNotSupported => 8,
            DeviceErrorCode::OutOfResources => 9,
            DeviceErrorCode::DeviceLockedByAnotherLink => 11,
            DeviceErrorCode::NoLockHeldByThisLink => 12,
            DeviceErrorCode::IoTimeout => 15,
            DeviceErrorCode::IoError => 17,
            DeviceErrorCode::InvalidAddress => 21,
            DeviceErrorCode::Abort => 23,
            DeviceErrorCode::ChannelAlreadyEstablished => 29,
            DeviceErrorCode::_Reserved(x) => *x,
        })
    }
}

impl XdrDecode for DeviceErrorCode {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
    where
        RD: Read,
    {
        let discriminant = reader.read_u32::<NetworkEndian>()?;
        *self = match discriminant {
            0 => DeviceErrorCode::NoError,
            1 => DeviceErrorCode::SyntaxError,
            3 => DeviceErrorCode::DeviceNotAccessible,
            4 => DeviceErrorCode::InvalidLinkIdentifier,
            5 => DeviceErrorCode::ParameterError,
            6 => DeviceErrorCode::ChannelNotEstablished,
            8 => DeviceErrorCode::OperationNotSupported,
            9 => DeviceErrorCode::OutOfResources,
            11 => DeviceErrorCode::DeviceLockedByAnotherLink,
            12 => DeviceErrorCode::NoLockHeldByThisLink,
            15 => DeviceErrorCode::IoTimeout,
            17 => DeviceErrorCode::IoError,
            21 => DeviceErrorCode::InvalidAddress,
            23 => DeviceErrorCode::Abort,
            29 => DeviceErrorCode::ChannelAlreadyEstablished,
            x => DeviceErrorCode::_Reserved(x),
        };
        Ok(())
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct DeviceError {
    pub(crate) error: DeviceErrorCode,
}

impl XdrEncode for DeviceError {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
    where
        WR: Write,
    {
        self.error.write_xdr(writer)
    }
}

impl XdrDecode for DeviceError {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
    where
        RD: Read,
    {
        self.error.read_xdr(reader)
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct CreateLinkParms {
    pub(crate) client_id: i32,
    pub(crate) lock_device: bool,
    pub(crate) lock_timeout: u32,
    pub(crate) device: String,
}

impl XdrEncode for CreateLinkParms {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
    where
        WR: Write,
    {
        self.client_id.write_xdr(writer)?;
        self.lock_device.write_xdr(writer)?;
        self.lock_timeout.write_xdr(writer)?;
        self.device.write_xdr(writer)
    }
}

impl XdrDecode for CreateLinkParms {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
    where
        RD: Read,
    {
        self.client_id.read_xdr(reader)?;
        self.lock_device.read_xdr(reader)?;
        self.lock_timeout.read_xdr(reader)?;
        self.device.read_xdr(reader)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct CreateLinkResp {
    pub(crate) error: DeviceErrorCode,
    pub(crate) lid: DeviceLink,
    pub(crate) abort_port: u16,
    pub(crate) max_recv_size: u32,
}

impl XdrEncode for CreateLinkResp {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
    where
        WR: Write,
    {
        self.error.write_xdr(writer)?;
        self.lid.write_xdr(writer)?;
        self.abort_port.write_xdr(writer)?;
        self.max_recv_size.write_xdr(writer)
    }
}

impl XdrDecode for CreateLinkResp {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
    where
        RD: Read,
    {
        self.error.read_xdr(reader)?;
        self.lid.read_xdr(reader)?;
        self.abort_port.read_xdr(reader)?;
        self.max_recv_size.read_xdr(reader)
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct DeviceWriteParms {
    lid: DeviceLink,
    io_timeout: u32,
    lock_timeout: u32,
    flags: DeviceFlags, //u16,
    data: Vec<u8>,
}

impl XdrEncode for DeviceWriteParms {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
    where
        WR: Write,
    {
        self.lid.write_xdr(writer)?;
        self.io_timeout.write_xdr(writer)?;
        self.lock_timeout.write_xdr(writer)?;
        self.flags.write_xdr(writer)?;
        self.data.write_xdr(writer)
    }
}

impl XdrDecode for DeviceWriteParms {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
    where
        RD: Read,
    {
        self.lid.read_xdr(reader)?;
        self.io_timeout.read_xdr(reader)?;
        self.lock_timeout.read_xdr(reader)?;
        self.flags.read_xdr(reader)?;
        self.data.read_xdr(reader)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct DeviceWriteResp {
    error: DeviceErrorCode,
    size: u32,
}

impl XdrEncode for DeviceWriteResp {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
    where
        WR: Write,
    {
        self.error.write_xdr(writer)?;
        self.size.write_xdr(writer)
    }
}

impl XdrDecode for DeviceWriteResp {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
    where
        RD: Read,
    {
        self.error.read_xdr(reader)?;
        self.size.read_xdr(reader)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct DeviceReadParms {
    lid: DeviceLink,
    request_size: u32,
    io_timeout: u32,
    lock_timeout: u32,
    flags: DeviceFlags, //u16,
    term_char: u32,     //u8
}

impl XdrEncode for DeviceReadParms {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
    where
        WR: Write,
    {
        self.lid.write_xdr(writer)?;
        self.request_size.write_xdr(writer)?;
        self.io_timeout.write_xdr(writer)?;
        self.lock_timeout.write_xdr(writer)?;
        self.flags.write_xdr(writer)?;
        self.term_char.write_xdr(writer)
    }
}

impl XdrDecode for DeviceReadParms {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
    where
        RD: Read,
    {
        self.lid.read_xdr(reader)?;
        self.request_size.read_xdr(reader)?;
        self.io_timeout.read_xdr(reader)?;
        self.lock_timeout.read_xdr(reader)?;
        self.flags.read_xdr(reader)?;
        self.term_char.read_xdr(reader)
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct DeviceReadResp {
    error: DeviceErrorCode,
    reason: u32,
    data: Vec<u8>,
}

impl XdrEncode for DeviceReadResp {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
    where
        WR: Write,
    {
        self.error.write_xdr(writer)?;
        self.reason.write_xdr(writer)?;
        self.data.write_xdr(writer)
    }
}

impl XdrDecode for DeviceReadResp {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
    where
        RD: Read,
    {
        self.error.read_xdr(reader)?;
        self.reason.read_xdr(reader)?;
        self.data.read_xdr(reader)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct DeviceReadStbResp {
    error: DeviceErrorCode,
    stb: u32,
}

impl XdrEncode for DeviceReadStbResp {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
    where
        WR: Write,
    {
        self.error.write_xdr(writer)?;
        self.stb.write_xdr(writer)
    }
}

impl XdrDecode for DeviceReadStbResp {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
    where
        RD: Read,
    {
        self.error.read_xdr(reader)?;
        self.stb.read_xdr(reader)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct DeviceGenericParms {
    lid: DeviceLink,
    flags: DeviceFlags,
    lock_timeout: u32,
    io_timeout: u32,
}

impl XdrEncode for DeviceGenericParms {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
    where
        WR: Write,
    {
        self.lid.write_xdr(writer)?;
        self.flags.write_xdr(writer)?;
        self.lock_timeout.write_xdr(writer)?;
        self.io_timeout.write_xdr(writer)
    }
}

impl XdrDecode for DeviceGenericParms {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
    where
        RD: Read,
    {
        self.lid.read_xdr(reader)?;
        self.flags.read_xdr(reader)?;
        self.lock_timeout.read_xdr(reader)?;
        self.io_timeout.read_xdr(reader)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct DeviceRemoteFunc {
    host_addr: u32,
    host_port: u16,
    prog_num: u32,
    prog_vers: u32,
    prog_family: DeviceAddrFamily,
}

impl XdrEncode for DeviceRemoteFunc {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
    where
        WR: Write,
    {
        self.host_addr.write_xdr(writer)?;
        self.host_port.write_xdr(writer)?;
        self.prog_num.write_xdr(writer)?;
        self.prog_vers.write_xdr(writer)?;
        self.prog_family.write_xdr(writer)
    }
}

impl XdrDecode for DeviceRemoteFunc {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
    where
        RD: Read,
    {
        self.host_addr.read_xdr(reader)?;
        self.host_port.read_xdr(reader)?;
        self.prog_num.read_xdr(reader)?;
        self.prog_vers.read_xdr(reader)?;
        self.prog_family.read_xdr(reader)
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct DeviceEnableSrqParms {
    lid: DeviceLink,
    enable: bool,
    handle: Vec<u8>,
}

impl XdrEncode for DeviceEnableSrqParms {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
    where
        WR: Write,
    {
        self.lid.write_xdr(writer)?;
        self.enable.write_xdr(writer)?;
        self.handle.write_xdr(writer)
    }
}

impl XdrDecode for DeviceEnableSrqParms {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
    where
        RD: Read,
    {
        self.lid.read_xdr(reader)?;
        self.enable.read_xdr(reader)?;
        self.handle.read_xdr(reader)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct DeviceLockParms {
    lid: DeviceLink,
    flags: DeviceFlags,
    lock_timeout: u32,
}

impl XdrEncode for DeviceLockParms {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
    where
        WR: Write,
    {
        self.lid.write_xdr(writer)?;
        self.flags.write_xdr(writer)?;
        self.lock_timeout.write_xdr(writer)
    }
}

impl XdrDecode for DeviceLockParms {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
    where
        RD: Read,
    {
        self.lid.read_xdr(reader)?;
        self.flags.read_xdr(reader)?;
        self.lock_timeout.read_xdr(reader)
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct DeviceDocmdParms {
    lid: DeviceLink,
    flags: DeviceFlags,
    io_timeout: u32,
    lock_timeout: u32,
    cmd: i32,
    network_order: bool,
    datasize: u32,
    data_in: Vec<u8>,
}

impl XdrEncode for DeviceDocmdParms {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
    where
        WR: Write,
    {
        self.lid.write_xdr(writer)?;
        self.flags.write_xdr(writer)?;
        self.io_timeout.write_xdr(writer)?;
        self.lock_timeout.write_xdr(writer)?;
        self.cmd.write_xdr(writer)?;
        self.network_order.write_xdr(writer)?;
        self.datasize.write_xdr(writer)?;
        self.data_in.write_xdr(writer)
    }
}

impl XdrDecode for DeviceDocmdParms {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
    where
        RD: Read,
    {
        self.lid.read_xdr(reader)?;
        self.flags.read_xdr(reader)?;
        self.io_timeout.read_xdr(reader)?;
        self.lock_timeout.read_xdr(reader)?;
        self.cmd.read_xdr(reader)?;
        self.network_order.read_xdr(reader)?;
        self.datasize.read_xdr(reader)?;
        self.data_in.read_xdr(reader)
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct DeviceDocmdResp {
    error: DeviceErrorCode,
    data_out: Vec<u8>,
}

impl XdrEncode for DeviceDocmdResp {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
    where
        WR: Write,
    {
        self.error.write_xdr(writer)?;
        self.data_out.write_xdr(writer)
    }
}

impl XdrDecode for DeviceDocmdResp {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
    where
        RD: Read,
    {
        self.error.read_xdr(reader)?;
        self.data_out.read_xdr(reader)
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct DeviceSrqParms {
    pub(crate) handle: Vec<u8>,
}

impl DeviceSrqParms {
    pub(crate) fn new(handle: Vec<u8>) -> Self {
        Self { handle }
    }
}

impl XdrEncode for DeviceSrqParms {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
    where
        WR: Write,
    {
        self.handle.write_xdr(writer)
    }
}

impl XdrDecode for DeviceSrqParms {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
    where
        RD: Read,
    {
        self.handle.read_xdr(reader)
    }
}
