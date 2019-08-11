//! Manual implementations of [`read_exact`](https://docs.rs/tokio/0.1.22/tokio/io/fn.read_exact.html)
//! and
//! [`write_all`](https://docs.rs/tokio/0.1.22/tokio/io/fn.write_all.html)
//!
//! Source: [https://tokio.rs/docs/io/async_read_write/](https://tokio.rs/docs/io/async_read_write/)

use futures::try_ready;
use std::io;
use tokio::prelude::*;

/// Future that reads from stream of type `R` into buffer of type `T`.
///
/// In the common case, this is set to `Some(Reading)`, but it is set to `None`
/// when it returns `Async::Ready` so that it can return the reader and the
/// buffer.
pub struct ReadExact<R, T>(Option<Reading<R, T>>);

/// Underlying struct of `ReadExact` containing the reader and the buffer.
pub struct Reading<R, T> {
    /// Stream to read from.
    pub reader: R,
    /// Buffer to write to.
    pub buffer: T,
    /// How far the buffer has been written into.
    pos: usize,
}

/// Constructs an Future that reads from `reader` and writes to `buffer`.
pub fn read_exact<R, T>(reader: R, buffer: T) -> ReadExact<R, T>
where
    R: AsyncRead,
    T: AsMut<[u8]>,
{
    ReadExact(Some(Reading {
        reader,
        buffer,
        pos: 0,
    }))
}

impl<R, T> Future for ReadExact<R, T>
where
    R: AsyncRead,
    T: AsMut<[u8]>,
{
    // Return both the reader and the buffer with the data once the buffer is
    // filled.
    type Item = (R, T);
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0 {
            Some(Reading {
                ref mut reader,
                ref mut buffer,
                ref mut pos,
            }) => {
                // Converts buffer to `&mut [u8]` slice to allow `len` method.
                let buffer = buffer.as_mut();
                while *pos < buffer.len() {
                    // Bubbles up `NotReady` or error from `poll_read`
                    // Otherwise, assigns `n` to number of bytes read.
                    let n = try_ready!(reader.poll_read(&mut buffer[*pos..]));
                    *pos += n;

                    // No bytes were read, implying no more data.
                    if n == 0 {
                        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "early eof"));
                    }
                }
            }
            None => panic!("poll a ReadExact after it's done"),
        }

        // Moves `Reading` out of `ReadExact` so that fields can be returned.
        let reading = self.0.take().expect("must have seen Some above");
        Ok(Async::Ready((reading.reader, reading.buffer)))
    }
}

/// A Future that writes from writer `W` to buffer `T`.
pub struct WriteAll<W, T>(Option<Writing<W, T>>);

/// Underlying struct of `WriteAll` containing the writer and the buffer.
pub struct Writing<W, T> {
    /// Stream to write to.
    pub writer: W,
    /// Buffer to read from.
    pub buffer: T,
    /// How far the buffer has been written from.
    pub pos: usize,
}

/// Constructs an Future that writes to `writer` from `buffer`.
pub fn write_all<W, T>(writer: W, buffer: T) -> WriteAll<W, T>
where
    W: AsyncWrite,
    T: AsRef<[u8]>,
{
    WriteAll(Some(Writing {
        writer,
        buffer,
        pos: 0,
    }))
}

impl<W, T> Future for WriteAll<W, T>
where
    W: AsyncWrite,
    T: AsRef<[u8]>,
{
    type Item = (W, T);
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0 {
            Some(Writing {
                ref mut writer,
                ref buffer,
                ref mut pos,
            }) => {
                let buffer = buffer.as_ref();
                while *pos < buffer.len() {
                    let n = try_ready!(writer.poll_write(&buffer[*pos..]));
                    *pos += n;

                    // No bytes were written, implying strange error.
                    if n == 0 {
                        return Err(io::Error::new(
                            io::ErrorKind::WriteZero,
                            "zero-length write",
                        ));
                    }
                }
            }
            None => panic!("poll a WriteAll after it's done"),
        }
        let writing = self.0.take().expect("must have seen Some above");
        Ok(Async::Ready((writing.writer, writing.buffer)))
    }
}
