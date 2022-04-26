
/// VXI-11 async channel program number
pub(crate) const DEVICE_ASYNC: u32 = 0x0607B0;

/// VXI-11 async channel program version
pub(crate) const DEVICE_ASYNC_VERSION: u32 = 1;

/// VXI-11 core channel program number
pub(crate) const DEVICE_CORE: u32 = 0x0607AF;

/// VXI-11 core channel program version
pub(crate) const DEVICE_CORE_VERSION: u32 = 1;

/// VXI-11 interrupt channel program number
pub(crate) const DEVICE_INTR: u32 = 0x0607B1;

/// VXI-11 interrupt channel program version
pub(crate) const DEVICE_INTR_VERSION: u32 = 1;

pub(crate) mod xdr {
    use std::io::{Read, Result, Write};

    use crate::common::xdr::prelude::*;

    #[derive(Debug, Default, Clone, Copy)]
    pub(crate) struct DeviceLink(pub u32);

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

    #[derive(Debug, Clone, Copy)]
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
        _Reserved(u32)
    }

    impl XdrEncode for DeviceErrorCode {
        fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()> where WR: Write {
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
                DeviceErrorCode::_Reserved(x) => *x
            })
        }
    }

    impl XdrDecode for DeviceErrorCode {
        fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()> where RD: Read {
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
                x => DeviceErrorCode::_Reserved(x)
            };
            Ok(())
        }
    }

}