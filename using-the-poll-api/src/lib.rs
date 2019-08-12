//! Possible implementation of the `read_exact` future for a `TcpStream`.
//!
//! Source: [https://tokio.rs/docs/io/async_read_write/](https://tokio.rs/docs/io/async_read_write/)
use futures::try_ready;
use std::mem;
use tokio::io;
use tokio::net::TcpStream;
use tokio::prelude::*;

/// A future which can be used to easily read exactly enough bytes to fill a
/// buffer.
///
/// Created by the `read_exact` function.
pub struct ReadExact {
    state: State,
}

/// Tracks the state of `ReadExact`.
enum State {
    /// Common case when bytes are still being read to the buffer.
    Reading {
        /// The stream read from.
        stream: TcpStream,
        /// The buffer being read to.
        buf: Vec<u8>,
        /// Number of bytes written to the buffer.
        pos: usize,
    },
    /// Allows `stream` and `buf` fields to be moved out when returning
    /// `poll` if the reading is finished.
    Empty,
}

impl Future for ReadExact {
    type Item = (TcpStream, Vec<u8>);
    type Error = io::Error;

    fn poll(&mut self) -> Result<Async<Self::Item>, io::Error> {
        match self.state {
            State::Reading {
                ref mut stream,
                ref mut buf,
                ref mut pos,
            } => {
                while *pos < buf.len() {
                    let n = try_ready!({ stream.poll_read(&mut buf[*pos..]) });
                    *pos += n;
                    if n == 0 {
                        let err = io::Error::new(io::ErrorKind::UnexpectedEof, "early eof");
                        return Err(err);
                    }
                }
            }
            State::Empty => panic!("poll a ReadExact after it's done"),
        }

        // Moves State::Empty to self.state, returning self.state
        match mem::replace(&mut self.state, State::Empty) {
            State::Reading { stream, buf, .. } => Ok(Async::Ready((stream, buf))),
            State::Empty => panic!(),
        }
    }
}

#[allow(dead_code)]
fn read_exact(stream: TcpStream, buf: Vec<u8>) -> ReadExact {
    ReadExact {
        state: State::Reading {
            stream,
            buf,
            pos: 0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn read_exact_usage() {
        let addr = "127.0.0.1:12345".parse().unwrap();
        let stream = TcpStream::connect(&addr).wait().unwrap();
        let _read_exact = read_exact(stream, vec![]);
    }
}
