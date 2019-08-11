//! Source: [https://tokio.rs/docs/io/reading_writing_data/](https://tokio.rs/docs/io/reading_writing_data/)
//!
//! Simple implementation of a line-based codec.
use bytes::{BufMut, BytesMut};
use tokio::codec::{Decoder, Encoder};

/// Keeps track of any extra book-keeping information the transport needs to
/// operate.
pub struct LinesCodec;

/// Turns string errors into `std::io::Error`
pub fn bad_utf8<E>(_: E) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "Unable to decode input as UTF8",
    )
}

impl Encoder for LinesCodec {
    type Item = String;
    type Error = std::io::Error;

    /// Writes out the bytes of the string followed by a new line.
    fn encode(&mut self, line: Self::Item, buf: &mut BytesMut) -> Result<(), Self::Error> {
        buf.reserve(line.len() + 1);
        buf.put(line);
        buf.put_u8(b'\n');
        Ok(())
    }
}

impl Decoder for LinesCodec {
    type Item = String;
    type Error = std::io::Error;

    /// Finds the next line in `buf`.
    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Ok(if let Some(offset) = buf.iter().position(|b| *b == b'\n') {
            // `offset` is the position of the newline character in `buf`.
            // Cuts out the line from `buf`, including the newline character.
            let line = buf.split_to(offset + 1);
            // Parses it as UTF-8.
            Some(
                // Now that newline character has been removed from buffer,
                // remove it from line as well.
                std::str::from_utf8(&line[..line.len() - 1])
                    // Maps `Utf8Error` to `std::io::Error`, returns early.
                    .map_err(bad_utf8)?
                    .to_string(),
            )
        } else {
            None
        })
    }

    /// Finds the next line in data `buf` when there will be no more data
    /// coming.
    fn decode_eof(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Ok(match self.decode(buf)? {
            Some(frame) => {
                // There is a regular line here, so just return it.
                Some(frame)
            }
            None => {
                // There are no more lines in `buf`.
                // There may be some remainder though.
                if buf.is_empty() {
                    // No remainder.
                    None
                } else {
                    Some(
                        std::str::from_utf8(&buf[..])
                            // Maps `Utf8Error` to `std::io::Error`, returns early.
                            .map_err(bad_utf8)?
                            .to_string(),
                    )
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_and_decodes_lines() -> Result<(), std::io::Error> {
        let mut transport = LinesCodec;
        let mut buf = BytesMut::new();
        let lines = "This is one line\nThis is another line\nThis is the final one".to_string();
        transport.encode(lines, &mut buf)?;

        let decoded = transport.decode_eof(&mut buf)?.unwrap();
        assert_eq!("This is one line".to_string(), decoded);
        let decoded = transport.decode_eof(&mut buf)?.unwrap();
        assert_eq!("This is another line".to_string(), decoded);
        let decoded = transport.decode_eof(&mut buf)?.unwrap();
        assert_eq!("This is the final one".to_string(), decoded);
        let decoded = transport.decode_eof(&mut buf)?;
        assert_eq!(None, decoded);
        Ok(())
    }
}
