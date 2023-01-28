use std::io;
use byteorder::{WriteBytesExt, ReadBytesExt};

pub enum Descriptor {
    SupportedTlsVersions(Vec<u16>),
    TlsInformation(Vec<u8>),
    TlsLastError(Vec<u8>),
    Reserved(u8, Vec<u8>),
    VendorSpecific(u8, Vec<u8>),
}

impl Descriptor {
    pub fn read_descriptor<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let len = reader.read_u16::<byteorder::NetworkEndian>()?;
        let typ = reader.read_u8()?;
        match typ {
            0 => {
                let mut buf = Vec::with_capacity(len as usize);
                for _ in 0..len {
                    buf.push(reader.read_u16::<byteorder::NetworkEndian>()?)
                }
                Ok(Self::SupportedTlsVersions(buf))
            },
            1 => {
                let mut buf = Vec::with_capacity(len as usize);
                reader.read_exact(&mut buf)?;
                Ok(Self::TlsInformation(buf))
            },
            2 => {
                let mut buf = Vec::with_capacity(len as usize);
                reader.read_exact(&mut buf)?;
                Ok(Self::TlsLastError(buf))
            },
            3..=127 => {
                let mut buf = Vec::with_capacity(len as usize);
                reader.read_exact(&mut buf)?;
                Ok(Self::Reserved(typ, buf))
            }
            128..=255 => {
                let mut buf = Vec::with_capacity(len as usize);
                reader.read_exact(&mut buf)?;
                Ok(Self::VendorSpecific(typ, buf))
            }
        }
    }

    pub fn write_descriptor<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        match self {
            Descriptor::SupportedTlsVersions(versions) => {
                writer.write_u16::<byteorder::NetworkEndian>((versions.len()*2) as u16)?;
                writer.write_u8(0)?;
                for v in versions {
                    writer.write_u16::<byteorder::NetworkEndian>(*v)?;
                }
            }
            Descriptor::TlsInformation(info) => {
                writer.write_u16::<byteorder::NetworkEndian>(info.len() as u16)?;
                writer.write_u8(1)?;
                writer.write(info)?;
            }
            Descriptor::TlsLastError(err) => {
                writer.write_u16::<byteorder::NetworkEndian>(err.len() as u16)?;
                writer.write_u8(2)?;
                writer.write(err)?;
            }
            Descriptor::Reserved(t, dat) | Descriptor::VendorSpecific(t, dat) => {
                writer.write_u16::<byteorder::NetworkEndian>(dat.len() as u16)?;
                writer.write_u8(t.clone())?;
                writer.write(dat)?;
            }
        }
        Ok(())
    }
}
