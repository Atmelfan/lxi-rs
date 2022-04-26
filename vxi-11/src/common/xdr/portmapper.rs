//! 


/// TCP port to use for portmapper/rpcbind
pub(crate) const PORTMAPPER_PORT: u16 = 111;

// Program constants
/// Portmapper program number
pub(crate) const PORTMAPPER_PROG: u32 = 100000;
// Portmapper program version
pub(crate) const PORTMAPPER_VERS: u32 = 2;

// Procedures
/// Null procedure
pub(crate) const PMAPPROC_NULL: u32 = 0;
/// Set procedure
pub(crate) const PMAPPROC_SET: u32 = 1;
/// Unset procedure
pub(crate) const PMAPPROC_UNSET: u32 = 2;
/// Getport procedure
pub(crate) const PMAPPROC_GETPORT: u32 = 3;
/// Dump procedure
pub(crate) const PMAPPROC_DUMP: u32 = 4;
/// Callit procedure
pub(crate) const PMAPPROC_CALLIT: u32 = 5;

/// Portmapper XDR types, see [RFC1833](https://datatracker.ietf.org/doc/html/rfc1833).
/// 
pub(crate) mod xdr {
    use std::io::{Read, Result, Write};

    use crate::common::xdr::prelude::*;


    #[derive(Debug, Default, Clone, Copy)]
    pub(crate) struct Mapping {
        prog: u32,
        vers: u32,
        prot: u32,
        port: u32,
    }

    impl XdrEncode for Mapping {
        fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
        where
            WR: Write,
        {
            self.prog.write_xdr(writer)?;
            self.vers.write_xdr(writer)?;
            self.prot.write_xdr(writer)?;
            self.port.write_xdr(writer)
        }
    }

    impl XdrDecode for Mapping {
        fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
        where
            RD: Read,
        {
            self.prog.read_xdr(reader)?;
            self.vers.read_xdr(reader)?;
            self.prot.read_xdr(reader)?;
            self.port.read_xdr(reader)
        }
    }

    /// The original struct is an option in its self and self-referencing. Wtf?
    #[derive(Debug, Default, Clone)]
    pub(crate) struct PmapList {
        list: Vec<Mapping>,
    }

    impl XdrEncode for PmapList {
        fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
        where
            WR: Write,
        {
            for x in &self.list {
                true.write_xdr(writer)?;
                x.write_xdr(writer)?;
            }
            false.write_xdr(writer)
        }
    }

    impl XdrDecode for PmapList {
        fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
        where
            RD: Read,
        {
            loop {
                let mut next: Option<Mapping> = None;
                next.read_xdr(reader)?;
                if let Some(mapping) = next {
                    self.list.push(mapping);
                } else {
                    break Ok(());
                }
            }
        }
    }

    #[derive(Debug, Default, Clone)]
    pub(crate) struct Callit {
        prog: u32,
        vers: u32,
        proc: u32,
        args: Vec<u8>,
    }

    impl XdrEncode for Callit {
        fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
        where
            WR: Write,
        {
            self.prog.write_xdr(writer)?;
            self.vers.write_xdr(writer)?;
            self.proc.write_xdr(writer)?;
            self.args.write_xdr(writer)
        }
    }

    impl XdrDecode for Callit {
        fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
        where
            RD: Read,
        {
            self.prog.read_xdr(reader)?;
            self.vers.read_xdr(reader)?;
            self.proc.read_xdr(reader)?;
            self.args.read_xdr(reader)
        }
    }

    #[derive(Debug, Default, Clone)]
    pub(crate) struct CallResult {
        port: u32,
        res: Vec<u8>,
    }

    impl XdrEncode for CallResult {
        fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()>
        where
            WR: Write,
        {
            self.port.write_xdr(writer)?;
            self.res.write_xdr(writer)
        }
    }

    impl XdrDecode for CallResult {
        fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()>
        where
            RD: Read,
        {
            self.port.read_xdr(reader)?;
            self.res.read_xdr(reader)
        }
    }

}

