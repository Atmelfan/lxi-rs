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
    ($reader:expr, $len:expr) => {
        let pad = (4 - ($len & 3)) & 3;
        for _ in 0..pad {
            let _ = $reader.read_u8()?;
        }
    };
}

macro_rules! write_padding {
    ($writer:expr, $len:expr) => {
        let pad = (4 - ($len & 3)) & 3;
        for _ in 0..pad {
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

impl XdrDecode for () {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()> where RD: Read {
        Ok(())
    }
}

impl XdrEncode for () {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()> where WR: Write {
        Ok(())
    }
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

impl XdrDecode for i16 {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()> where RD: Read {
        *self = reader.read_i32::<NetworkEndian>()? as Self;
        Ok(())
    }
}

impl XdrEncode for i16 {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()> where WR: Write {
        writer.write_i32::<NetworkEndian>(*self as i32)
    }
}

impl XdrDecode for i8 {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()> where RD: Read {
        *self = reader.read_i32::<NetworkEndian>()? as Self;
        Ok(())
    }
}

impl XdrEncode for i8 {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()> where WR: Write {
        writer.write_i32::<NetworkEndian>(*self as i32)
    }
}

#[cfg(test)]
mod test_xdr_integer {
    use std::io::Cursor;

    use super::{XdrDecode, XdrEncode};


    #[test]
    fn decode() {
        let mut cursor = Cursor::new(b"\xff\xff\xff\xfe");
        let mut i: i32 = 0;
        i.read_xdr(&mut cursor).unwrap();

        assert_eq!(i, -2)
    }

    #[test]
    fn encode() {
        let mut cursor = Cursor::new(Vec::new());
        let i: i32 = -2;
        i.write_xdr(&mut cursor).unwrap();
        
        assert_eq!(cursor.get_ref()[..], b"\xff\xff\xff\xfe"[..])

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

impl XdrDecode for u16 {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()> where RD: Read {
        *self = reader.read_u32::<NetworkEndian>()? as Self;
        Ok(())
    }
}

impl XdrEncode for u16 {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()> where WR: Write {
        writer.write_u32::<NetworkEndian>(*self as u32)
    }
}

impl XdrDecode for u8 {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()> where RD: Read {
        *self = reader.read_u32::<NetworkEndian>()? as Self;
        Ok(())
    }
}

impl XdrEncode for u8 {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()> where WR: Write {
        writer.write_u32::<NetworkEndian>(*self as u32)
    }
}

#[cfg(test)]
mod test_xdr_unsigned {
    use std::io::Cursor;

    use super::{XdrDecode, XdrEncode};


    #[test]
    fn decode() {
        let mut cursor = Cursor::new(b"\x00\x00\x00\x01");
        let mut i: u32 = 0;
        i.read_xdr(&mut cursor).unwrap();

        assert_eq!(i, 1)
    }

    #[test]
    fn encode() {
        let mut cursor = Cursor::new(Vec::new());
        let i: u32 = 1;
        i.write_xdr(&mut cursor).unwrap();
        
        assert_eq!(cursor.get_ref()[..], b"\x00\x00\x00\x01"[..])

    }
}

// 4.3 Enumerations
// Manually implemented where needed

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

#[cfg(test)]
mod test_xdr_boolean {
    use std::io::Cursor;

    use super::{XdrDecode, XdrEncode};


    #[test]
    fn decode() {
        let mut cursor = Cursor::new(b"\x00\x00\x00\x01");
        let mut i: bool = false;
        i.read_xdr(&mut cursor).unwrap();

        assert_eq!(i, true)
    }

    #[test]
    fn encode() {
        let mut cursor = Cursor::new(Vec::new());
        let i: bool = true;
        i.write_xdr(&mut cursor).unwrap();
        
        assert_eq!(cursor.get_ref()[..], b"\x00\x00\x00\x01"[..])

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

#[cfg(test)]
mod test_xdr_hyper {
    use std::io::Cursor;

    use super::{XdrDecode, XdrEncode};


    #[test]
    fn decode() {
        let mut cursor = Cursor::new(b"\x00\x00\x00\x00\x00\x00\x00\x01");
        let mut i: u64 = 0;
        i.read_xdr(&mut cursor).unwrap();

        assert_eq!(i, 1)
    }

    #[test]
    fn encode() {
        let mut cursor = Cursor::new(Vec::new());
        let i: u64 = 1;
        i.write_xdr(&mut cursor).unwrap();
        
        assert_eq!(cursor.get_ref()[..], b"\x00\x00\x00\x00\x00\x00\x00\x01"[..])

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

#[cfg(test)]
mod test_xdr_float {
    use std::io::Cursor;

    use super::{XdrDecode, XdrEncode};


    #[test]
    fn decode() {
        let mut cursor = Cursor::new(b"\x40\x49\x0e\x56");
        let mut i: f32 = 0.0;
        i.read_xdr(&mut cursor).unwrap();

        assert_eq!(i, 3.1415)
    }

    #[test]
    fn encode() {
        let mut cursor = Cursor::new(Vec::new());
        let i: f32 = 3.1415;
        i.write_xdr(&mut cursor).unwrap();
        
        assert_eq!(cursor.get_ref()[..], b"\x40\x49\x0e\x56"[..])

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

#[cfg(test)]
mod test_xdr_double {
    use std::io::Cursor;

    use super::{XdrDecode, XdrEncode};


    #[test]
    fn decode() {
        let mut cursor = Cursor::new(b"\x40\x09\x21\xca\xc0\x83\x12\x6f");
        let mut i: f64 = 0.0;
        i.read_xdr(&mut cursor).unwrap();

        assert_eq!(i, 3.1415)
    }

    #[test]
    fn encode() {
        let mut cursor = Cursor::new(Vec::new());
        let i: f64 = 3.1415;
        i.write_xdr(&mut cursor).unwrap();
        
        assert_eq!(cursor.get_ref()[..], b"\x40\x09\x21\xca\xc0\x83\x12\x6f"[..])

    }
}

// 4.8 Quadruple-Precision Floating-Point
// Nobody uses this

// 4.9 Fixed-Length Opaque Data
// impl<const N: usize> XdrDecode for [u8; N] {
//     fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()> where RD: Read {
//         reader.read_exact(self)?;
//         read_padding!(reader, N);
//         Ok(())
//     }
// }

// impl<const N: usize> XdrEncode for [u8; N] {
//     fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()> where WR: Write {
//         writer.write_all(self)?;
//         write_padding!(writer, N);
//         Ok(())
//     }
// }

#[cfg(test)]
mod test_xdr_opaque {
    use std::io::Cursor;

    use super::{XdrDecode, XdrEncode};


    #[test]
    fn decode() {
        let mut cursor = Cursor::new(b"\x01\x02\x00\x00");
        let mut i = [0u8; 2];
        i.read_xdr(&mut cursor).unwrap();

        assert_eq!(i, [1u8, 2u8]);

        let mut cursor = Cursor::new(b"\x01\x02\x03\x04");
        let mut i = [0u8; 4];
        i.read_xdr(&mut cursor).unwrap();

        assert_eq!(i, [1u8, 2u8, 3u8, 4u8])
    }

    #[test]
    fn encode() {
        let mut cursor = Cursor::new(Vec::new());
        let i = [1u8, 2u8];
        i.write_xdr(&mut cursor).unwrap();
        
        assert_eq!(cursor.get_ref()[..], b"\x01\x02\x00\x00"[..]);

        let mut cursor = Cursor::new(Vec::new());
        let i = [1u8, 2u8, 3u8, 4u8];
        i.write_xdr(&mut cursor).unwrap();
        
        assert_eq!(cursor.get_ref()[..], b"\x01\x02\x03\x04"[..])

    }
}



// 4.10 Variable-Length Opaque Data
// impl XdrDecode for Vec<u8> {
//     fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()> where RD: Read {
//         let len = reader.read_u32::<NetworkEndian>()? as usize;
//         *self = vec![0; len];
//         reader.read_exact(self)?;
//         read_padding!(reader, len);
//         Ok(())
//     }
// }

// impl XdrEncode for Vec<u8> {
//     fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()> where WR: Write {
//         writer.write_u32::<NetworkEndian>(self.len() as u32)?;
//         writer.write_all(self)?;
//         write_padding!(writer, self.len());
//         Ok(())
//     }
// }

#[cfg(test)]
mod test_xdr_variable_opaque {
    use std::io::Cursor;

    use super::{XdrDecode, XdrEncode};


    #[test]
    fn decode() {
        let mut cursor = Cursor::new(b"\x00\x00\x00\x02\x01\x02\x00\x00");
        let mut i: Vec<u8> = Vec::new();
        i.read_xdr(&mut cursor).unwrap();

        assert_eq!(i, vec![1u8, 2u8]);

        let mut cursor = Cursor::new(b"\x00\x00\x00\x04\x01\x02\x03\x04");
        let mut i: Vec<u8> = Vec::new();
        i.read_xdr(&mut cursor).unwrap();

        assert_eq!(i, [1u8, 2u8, 3u8, 4u8])
    }

    #[test]
    fn encode() {
        let mut cursor = Cursor::new(Vec::new());
        let i = vec![1u8, 2u8];
        i.write_xdr(&mut cursor).unwrap();
        
        assert_eq!(cursor.get_ref()[..], b"\x00\x00\x00\x02\x01\x02\x00\x00"[..]);

        let mut cursor = Cursor::new(Vec::new());
        let i = vec![1u8, 2u8, 3u8, 4u8];
        i.write_xdr(&mut cursor).unwrap();
        
        assert_eq!(cursor.get_ref()[..], b"\x00\x00\x00\x04\x01\x02\x03\x04"[..])

    }
}

// 4.11  String
impl XdrDecode for String {
    fn read_xdr<RD>(&mut self, reader: &mut RD) -> Result<()> where RD: Read {
        let len = reader.read_u32::<NetworkEndian>()? as u64;
        let mut s = reader.take(len);
        s.read_to_string(self)?;
        read_padding!(reader, len);
        Ok(())
    }
}

impl XdrEncode for String {
    fn write_xdr<WR>(&self, writer: &mut WR) -> Result<()> where WR: Write {
        let bytes = self.as_bytes();
        writer.write_u32::<NetworkEndian>(bytes.len() as u32)?;
        writer.write_all(bytes)?;
        write_padding!(writer, self.len());
        Ok(())
    }
}

#[cfg(test)]
mod test_xdr_string {
    use std::io::Cursor;

    use super::{XdrDecode, XdrEncode};


    #[test]
    fn decode() {
        let mut cursor = Cursor::new(b"\x00\x00\x00\x02ab\x00\x00");
        let mut i = String::new();
        i.read_xdr(&mut cursor).unwrap();

        assert_eq!(i, "ab");

        let mut cursor = Cursor::new(b"\x00\x00\x00\x04abcd");
        let mut i = String::new();
        i.read_xdr(&mut cursor).unwrap();

        assert_eq!(i, "abcd");
    }

    #[test]
    fn encode() {
        let mut cursor = Cursor::new(Vec::new());
        let i = "ab".to_string();
        i.write_xdr(&mut cursor).unwrap();
        
        assert_eq!(cursor.get_ref()[..], b"\x00\x00\x00\x02ab\x00\x00"[..]);

        let mut cursor = Cursor::new(Vec::new());
        let i = "abcd".to_string();
        i.write_xdr(&mut cursor).unwrap();
        
        assert_eq!(cursor.get_ref()[..], b"\x00\x00\x00\x04abcd"[..])

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
        writer.write_u32::<NetworkEndian>(self.len() as u32)?;
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