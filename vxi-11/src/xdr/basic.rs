//! Basic types for XDR, see [RFC4506](https://datatracker.ietf.org/doc/html/rfc4506).
//! 
//! Provides the following types:
//! 
//! | XDR Type         | Rust type |
//! |------------------|-----------|
//! | integer          | i32       |
//! | unsigned integer | u32       |
//! | Boolean          | bool      |
//! | hyper            | i64       |
//! | unsigned hyper   | u64       |
//! | float            | f32       |
//! | double           | f64       |
//! | opaque[n]        | [u8; N]   |
//! | opaque<>         | Vec<u8>   |
//! | string<>         | String    |
//! | T ident[n]       | [T; N]    |
//! | T ident<n>       | Vec<T>    |
//! 
//! Enums and structures are implemented by deriving XdrEncode and XdrDecode.
//! 

use std::io::{Read, Result, Write};
use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};




macro_rules! read_padding {
    ($reader:expr, $padding:expr) => {
        for _ in 0..$padding {
            let _ = $reader.read_u8()?;
        }
    };
}

macro_rules! write_padding {
    ($writer:expr, $padding:expr) => {
        for _ in 0..$padding {
            $writer.write_u8(0)?;
        }
    };
}



pub trait XdrDecode {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()> where RD: Read;
}

pub trait XdrEncode {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()> where WR: Write;

}


// 4.1.  Integer
impl XdrDecode for i32 {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()> where RD: Read {
        *self = reader.read_i32::<NetworkEndian>()?;
        Ok(())
    }
}

impl XdrEncode for i32 {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()> where WR: Write {
        writer.write_i32::<NetworkEndian>(*self)
    }
}

// 4.2 Unsigned Integer
impl XdrDecode for u32 {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()> where RD: Read {
        *self = reader.read_u32::<NetworkEndian>()?;
        Ok(())
    }
}

impl XdrEncode for u32 {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()> where WR: Write {
        writer.write_u32::<NetworkEndian>(*self)
    }
}

// 4.3 Enumerations

// 4.4 Booleans
impl XdrDecode for bool {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()> where RD: Read {
        let x = reader.read_i32::<NetworkEndian>()?;
        if x == 0 {
            *self = false;
        } else {
            *self = true;
        }
        Ok(())
    }
}

impl XdrEncode for bool {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()> where WR: Write {
        if *self {
            writer.write_i32::<NetworkEndian>(1)
        } else {
            writer.write_i32::<NetworkEndian>(0)
        }
    }
}

// 4.5 Hyper Integer and Unsigned Hyper Integer
impl XdrDecode for u64 {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()> where RD: Read {
        *self = reader.read_u64::<NetworkEndian>()?;
        Ok(())
    }
}

impl XdrEncode for u64 {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()> where WR: Write {
        writer.write_u64::<NetworkEndian>(*self)
    }
}

impl XdrDecode for i64 {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()> where RD: Read {
        *self = reader.read_i64::<NetworkEndian>()?;
        Ok(())
    }
}

impl XdrEncode for i64 {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()> where WR: Write {
        writer.write_i64::<NetworkEndian>(*self)
    }
}

// 4.6 Floating-Point
impl XdrDecode for f32 {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()> where RD: Read {
        *self = reader.read_f32::<NetworkEndian>()?;
        Ok(())
    }
}

impl XdrEncode for f32 {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()> where WR: Write {
        writer.write_f32::<NetworkEndian>(*self)
    }
}

// 4.7 Double-Precision Floating-Point
impl XdrDecode for f64 {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()> where RD: Read {
        *self = reader.read_f64::<NetworkEndian>()?;
        Ok(())
    }
}

impl XdrEncode for f64 {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()> where WR: Write {
        writer.write_f64::<NetworkEndian>(*self)
    }
}

// 4.8 Quadruple-Precision Floating-Point
// Nobody uses this

// 4.9 Fixed-Length Opaque Data
impl<const N: usize> XdrDecode for [u8; N] {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()> where RD: Read {
        let pad = (4 - (N & 3)) & 3;
        reader.read_exact(self)?;
        read_padding!(reader, pad);
        Ok(())
    }
}

impl<const N: usize> XdrEncode for [u8; N] {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()> where WR: Write {
        let pad = (4 - (N & 3)) & 3;
        writer.write_all(self)?;
        write_padding!(writer, pad);
        Ok(())
    }
}

// 4.10 Variable-Length Opaque Data
impl XdrDecode for Vec<u8> {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()> where RD: Read {
        let len = reader.read_u32::<NetworkEndian>()? as usize;
        let pad = (4 - (len & 3)) & 3;
        *self = vec![0; len];
        reader.read_exact(self)?;
        read_padding!(reader, pad);
        Ok(())
    }
}

impl XdrEncode for Vec<u8> {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()> where WR: Write {
        let pad = (4 - (self.len() & 3)) & 3;
        writer.write_all(self)?;
        write_padding!(writer, pad);
        Ok(())
    }
}

// 4.11  String
impl XdrDecode for String {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()> where RD: Read {
        let len = reader.read_u32::<NetworkEndian>()? as u64;
        let pad = (4 - (len & 3)) & 3;
        let mut s = reader.take(len);
        s.read_to_string(self)?;
        read_padding!(reader, pad);
        Ok(())
    }
}

impl XdrEncode for String {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()> where WR: Write {
        let len = self.len();
        let pad = (4 - (len & 3)) & 3;
        writer.write_all(self.as_bytes())?;
        write_padding!(writer, pad);
        Ok(())
    }
}

// 4.12 Fixed-Length Array
impl<T: XdrDecode, const N: usize> XdrDecode for [T; N] {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()> where RD: Read {
        for x in self {
            x.read_xdr(reader)?;
        }
        Ok(())
    }
}

impl<T: XdrEncode, const N: usize> XdrEncode for [T; N] {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()> where WR: Write {
        for x in self {
            x.write_xdr(writer)?;
        }
        Ok(())
    }
}

// 4.13 Variable-Length Array
impl<T: XdrDecode + Default> XdrDecode for Vec<T> {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()> where RD: Read {
        let len = reader.read_u32::<NetworkEndian>()? as usize;
        for _ in 0..len {
            let mut x: T = Default::default();
            x.read_xdr(reader)?;
            self.push(x);
        }
        Ok(())
    }
}

impl<T: XdrEncode> XdrEncode for Vec<T> {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()> where WR: Write {
        for x in self {
            x.write_xdr(writer)?;
        }
        Ok(())
    }
}


// 4.19 Optional data
impl<T: XdrDecode + Default> XdrDecode for Option<T> {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()> where RD: Read {
        let mut x: bool = false;
        x.read_xdr(reader)?;
        *self = if x {
            let mut t: T = Default::default();
            t.read_xdr(reader)?;
            Some(t)
        } else {
            None
        };
        Ok(())
    }
}

impl<T: XdrEncode> XdrEncode for Option<T> {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()> where WR: Write {
        self.is_some().write_xdr(writer)?;
        if let Some(t) = self {
            t.write_xdr(writer)?;
        };
        Ok(())
    }
}