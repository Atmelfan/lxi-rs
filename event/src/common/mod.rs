use std::{
    io::{Cursor, ErrorKind, Read},
    time::{Duration, Instant},
};

use async_std::io::{Error, Read as AsyncRead, ReadExt, Write as AsyncWrite};
use bitfield::bitfield;
use byteorder::{ByteOrder, NetworkEndian, ReadBytesExt};

const DATATYPE_ASCII: i8 = -1;
const DATATYPE_INT8: i8 = -2;
const DATATYPE_UINT8: i8 = -3;
const DATATYPE_INT16: i8 = -4;
const DATATYPE_UINT16: i8 = -5;
const DATATYPE_INT32: i8 = -6;
const DATATYPE_UINT32: i8 = -7;
const DATATYPE_INT64: i8 = -8;
const DATATYPE_UINT64: i8 = -9;
const DATATYPE_FLOAT32: i8 = -10;
const DATATYPE_FLOAT64: i8 = -11;
const DATATYPE_FLOAT128: i8 = -12;
const DATATYPE_UTF8: i8 = -13;
const DATATYPE_UTF8JSON: i8 = -14;
const DATATYPE_UTF8XML: i8 = -15;
const DATATYPE_OCTETS: i8 = -16;

pub(crate) struct Message {
    domain: u8,
    event_id: [u8; 16],
    sequence: u32,
    epoch: u16,
    seconds: u32,
    nanoseconds: u32,
    fractional: u16,
    flags: Flags,
    data_fields: Vec<DataField>,
}

impl Message {
    const MESSAGE_HEADER_SIZE: usize = 3 + 1 + 16 + 4 + 10 + 2 + 2;

    pub fn get_timestamp_seconds(&self) -> u64 {
        (self.epoch as u64) << 16 | (self.seconds as u64)
    }

    pub fn get_timestamp_nanoseconds(&self) -> u64 {
        self.nanoseconds as u64
    }

    pub fn get_timestamp_fractional(&self) -> u16 {
        self.fractional
    }

    pub fn get_timestamp(&self) -> std::time::SystemTime {
        std::time::UNIX_EPOCH
            + Duration::from_secs(self.get_timestamp_seconds())
            + Duration::from_nanos(self.get_timestamp_nanoseconds())
    }

    pub(crate) async fn read_from<RD>(reader: &mut RD) -> Result<Message, Error>
    where
        RD: AsyncRead + Unpin,
    {
        let mut buf = [0u8; Message::MESSAGE_HEADER_SIZE];
        reader.read_exact(&mut buf).await?;
        let mut cursor = Cursor::new(&buf);

        // HW detect
        let mut hw_detect = [0u8; 3];
        cursor.read_exact(&mut hw_detect)?;
        if &hw_detect != b"LXI" {
            return Err(ErrorKind::Other.into());
        }

        // Domain
        let domain = cursor.read_u8()?;

        // Event ID
        let mut event_id = [0u8; 16];
        cursor.read_exact(&mut event_id)?;

        // Sequence
        let sequence = cursor.read_u32::<NetworkEndian>()?;

        // Timestamp
        let seconds = cursor.read_u32::<NetworkEndian>()?;
        let nanoseconds = cursor.read_u32::<NetworkEndian>()?;
        let fractional = cursor.read_u16::<NetworkEndian>()?;

        // Epoch
        let epoch = cursor.read_u16::<NetworkEndian>()?;

        // Flags
        let flags = cursor.read_u16::<NetworkEndian>()?;

        // Data fields
        let mut data_fields = Vec::new();
        loop {
            let mut lbuf = [0u8; 2];
            reader.read_exact(&mut lbuf).await?;
            let length = NetworkEndian::read_u16(&lbuf[..]);

            // End of data fields
            if length == 0 {
                break;
            }

            let mut data = Vec::new();
            reader
                .take((length as u64) + 1)
                .read_to_end(&mut data)
                .await?;

            let field = DataField::from_buffer(&data)?;
            data_fields.push(field);
        }

        Ok(Message {
            domain,
            event_id,
            sequence,
            epoch,
            seconds,
            nanoseconds,
            fractional,
            flags: Flags(flags),
            data_fields,
        })
    }

    pub(crate) async fn write_to<WR>(&self, writer: &mut WR) -> Result<(), Error>
    where
        WR: AsyncWrite + Unpin,
    {
        let mut buf = [0u8; Message::MESSAGE_HEADER_SIZE];
        buf[0] = b'L';
        buf[1] = b'X';
        buf[2] = b'I';
        buf[3] = self.domain;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TimeRepresentation {
    seconds: u32,
    nanoseconds: u32,
    fractional: u16,
}

impl TimeRepresentation {
    pub(crate) fn new(seconds: u32, nanoseconds: u32, fractional: u16) -> Self {
        Self {
            seconds,
            nanoseconds,
            fractional,
        }
    }
}

bitfield! {
    pub struct Flags(u16);
    impl Debug;
    // The fields default to u16
    pub error, set_error : 0;
    pub hardware, set_hardware : 2;
    pub acknowledge, set_acknowledge : 3;
    pub stateless, set_stateless : 4;
}

pub(crate) enum DataField {
    /// Unknown data type
    Unknown(i8, Vec<u8>),
    /// ASCII formatted text
    Ascii(Vec<u8>),
    Int8(Vec<i8>),
    Uint8(Vec<u8>),
    Int16(Vec<i16>),
    Uint16(Vec<u16>),
    Int32(Vec<i32>),
    Uint32(Vec<u32>),
    Int64(Vec<i64>),
    Uint64(Vec<u64>),
    Float32(Vec<f32>),
    Float64(Vec<f64>),
    /// Reading f128's are left as an excercise to user
    Float128(Vec<u8>),
    Utf8(String),
    Utf8Json(String),
    Utf8Xml(String),
    Octets(Vec<u8>),
}

macro_rules! read_data_type {
    ($cursor:expr, $func:ident, $len:expr) => {{
        let mut x = Vec::new();
        for _ in 0..$len {
            let n = $cursor.$func()?;
            x.push(n)
        }
        x
    }};
    ($cursor:expr, $func:ident, $len:expr, $endian:ident) => {{
        let mut x = Vec::new();
        for _ in 0..$len {
            let n = $cursor.$func::<$endian>()?;
            x.push(n)
        }
        x
    }};
}

impl DataField {
    fn from_buffer(buf: &[u8]) -> Result<Self, Error> {
        let typ = buf[0] as i8;
        let data = &buf[1..];
        let mut cursor = Cursor::new(data);
        let len = buf.len() - 1;
        match typ {
            self::DATATYPE_ASCII => Ok(Self::Ascii(data.to_vec())),
            DATATYPE_INT8 => {
                let x = read_data_type!(cursor, read_i8, len);
                Ok(Self::Int8(x))
            }
            DATATYPE_UINT8 => {
                let x = read_data_type!(cursor, read_u8, len);
                Ok(Self::Uint8(x))
            }
            DATATYPE_INT16 => {
                let x = read_data_type!(cursor, read_i16, len, NetworkEndian);
                Ok(Self::Int16(x))
            }
            DATATYPE_UINT16 => {
                let x = read_data_type!(cursor, read_u16, len, NetworkEndian);
                Ok(Self::Uint16(x))
            }
            DATATYPE_INT32 => {
                let x = read_data_type!(cursor, read_i32, len, NetworkEndian);
                Ok(Self::Int32(x))
            }
            DATATYPE_UINT32 => {
                let x = read_data_type!(cursor, read_u32, len, NetworkEndian);
                Ok(Self::Uint32(x))
            }
            DATATYPE_INT64 => {
                let x = read_data_type!(cursor, read_i64, len, NetworkEndian);
                Ok(Self::Int64(x))
            }
            DATATYPE_UINT64 => {
                let x = read_data_type!(cursor, read_u64, len, NetworkEndian);
                Ok(Self::Uint64(x))
            }
            DATATYPE_FLOAT32 => {
                let x = read_data_type!(cursor, read_f32, len, NetworkEndian);
                Ok(Self::Float32(x))
            }
            DATATYPE_FLOAT64 => {
                let x = read_data_type!(cursor, read_f64, len, NetworkEndian);
                Ok(Self::Float64(x))
            }
            DATATYPE_FLOAT128 => Ok(Self::Float128(data.to_vec())),
            DATATYPE_UTF8 => {
                let string =
                    String::from_utf8(data.to_vec()).map_err(|_| Error::from(ErrorKind::Other))?;
                Ok(Self::Utf8(string))
            }
            DATATYPE_UTF8JSON => {
                let string =
                    String::from_utf8(data.to_vec()).map_err(|_| Error::from(ErrorKind::Other))?;
                Ok(Self::Utf8Json(string))
            }
            DATATYPE_UTF8XML => {
                let string =
                    String::from_utf8(data.to_vec()).map_err(|_| Error::from(ErrorKind::Other))?;
                Ok(Self::Utf8Xml(string))
            }
            DATATYPE_OCTETS => Ok(Self::Octets(data.to_vec())),
            code @ _ => Ok(Self::Unknown(code, data.to_vec())),
        }
    }
}
