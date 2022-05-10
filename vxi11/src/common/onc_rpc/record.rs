use std::io::{ErrorKind, Result};

use byteorder::{ByteOrder, NetworkEndian};
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub(crate) async fn read_record<RD>(reader: &mut RD, maxlen: usize) -> Result<Vec<u8>>
where
    RD: AsyncRead + Unpin,
{
    let mut buf = Vec::new();

    loop {
        // Read record header
        let mut fragment_header = [0u8; 4];
        reader.read_exact(&mut fragment_header).await?;
        let fragment_len = NetworkEndian::read_u32(&fragment_header[..]);

        // Assemble record
        let len = (fragment_len & 0x7FFFFFFF) as usize;
        if buf.len() + len > maxlen || buf.try_reserve(len).is_err() {
            return Err(ErrorKind::OutOfMemory.into());
        }
        reader
            .take((fragment_len & 0x7FFFFFFF) as u64)
            .read_to_end(&mut buf)
            .await?;

        // Check if last fragment
        if fragment_len & 0x80000000 != 0 {
            break Ok(buf);
        }
    }
}

pub(crate) async fn write_record<WR>(writer: &mut WR, record: Vec<u8>) -> Result<()>
where
    WR: AsyncWrite + Unpin,
{
    // Write header
    let fragment_len: u32 = 0x80000000 | (record.len() & 0x7FFFFFFF) as u32;
    let mut fragment_header = [0u8; 4];
    NetworkEndian::write_u32(&mut fragment_header, fragment_len);
    writer.write_all(&fragment_header).await?;

    // Write record
    writer.write_all(record.as_slice()).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use futures::io::Cursor;

    #[async_std::test]
    async fn reassemble_single_fragment() {
        let mut cursor = Cursor::new(b"\x80\x00\x00\x04\x01\x02\x03\x04");
        let rec = super::read_record(&mut cursor, 10).await.unwrap();

        assert_eq!(rec[..], [1, 2, 3, 4])
    }

    #[async_std::test]
    async fn reassemble_multiple_fragment() {
        let mut cursor = Cursor::new(b"\x00\x00\x00\x02\x01\x02\x80\x00\x00\x02\x03\x04");
        let rec = super::read_record(&mut cursor, 10).await.unwrap();

        assert_eq!(rec[..], [1, 2, 3, 4])
    }
}
