//! Non-trival Tokio server application: a chat server.
//!
//! The server uses a line-based protocol. Lines are termined by `\r\n`. This
//! is compatible with telnet. When a client connects, it must identify itself
//! by sending a line containing its "nick", a name used to identify the client
//! among its peers.
//!
//! Once a client is identified, all sent lines are prefixed with `[nick]:` and
//! broadcasted to all other connected clients.
//!
//! # Implementation Details
//!
//! Messages recieved from one client are broadcasted to all other connected
//! clients through message passing over mpsc channels. Each client socket is
//! managed by a task. Each task has an associated mpsc channel that is used to
//! recieve messages from other clients. The send half of all these channels is
//! stored in an `Rc` cell in order to make them accessible.
//!
//! Uses *unbounded* channels for simplicity, but at the cost of allowing
//! backpressure, the built up of unprocessed data due to producers creating
//! more data than can be consumed by consumers.
//!
//! # Reference
//!
//! Source: [https://tokio.rs/docs/going-deeper/chat/](https://tokio.rs/docs/going-deeper/chat/)
//!
//! Full code: [https://github.com/tokio-rs/tokio/blob/v0.1.x/tokio/examples/chat.rs](https://github.com/tokio-rs/tokio/blob/v0.1.x/tokio/examples/chat.rs)
//!
//! Note that Tokio provides some additional abstractions that would reduce the
//! number of lines to write this chat server.

use bytes::{BufMut, Bytes, BytesMut};
use futures::sync::mpsc;
use futures::try_ready;
use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

/// Shorthand for the transmit half of the message channel.
type Tx = mpsc::UnboundedSender<Bytes>;
/// Shorthand for the transmit half of the message channel.
type Rx = mpsc::UnboundedReceiver<Bytes>;

/// Tracks the shared state.
struct Shared {
    /// Maps each socket address to a transmit half of the message channel.
    peers: HashMap<SocketAddr, Tx>,
}

impl Shared {
    /// Creates an initial shared state.
    fn new() -> Shared {
        Shared {
            peers: HashMap::new(),
        }
    }
}

/// Takes a byte stream and exposes a read and write API at frame level, where
/// a frame is seperated by `\r\n`.
struct Lines {
    /// Byte stream to read from and write to.
    socket: TcpStream,
    /// Buffer for data read from the socket.
    rd: BytesMut,
    /// Buffer for data to write to the socket.
    wr: BytesMut,
}

impl Lines {
    /// Create a new `Line` codec backed by the socket.
    ///
    /// `socket` is where `Line` will read from and write to.
    fn new(socket: TcpStream) -> Self {
        Lines {
            socket,
            rd: BytesMut::new(),
            wr: BytesMut::new(),
        }
    }

    /// Fills buffer with any new data that might have been received off the
    /// socket.
    fn fill_read_buf(&mut self) -> Result<Async<()>, io::Error> {
        loop {
            // Ensures that read buffer has capacity.
            self.rd.reserve(1024);
            // Read data into the buffer, returning early if `read_buf` is not
            // ready or errors.
            let n = try_ready!(self.socket.read_buf(&mut self.rd));

            // If number of bytes read is zero, then the socket "ready"
            // meaning all the data has been read, it needs to be closed.
            if n == 0 {
                return Ok(Async::Ready(()));
            }
        }
    }

    /// Push `lines` onto the end of the write buffer.
    fn buffer(&mut self, lines: &[u8]) {
        // Ensures that buffer has capacity for the line.
        self.wr.reserve(lines.len());
        self.wr.put(lines);
    }

    /// Attempts to flush the buffer and write to the socket.
    fn poll_flush(&mut self) -> Poll<(), io::Error> {
        // As long as there is buffered data to write, attempt to write it.
        while !self.wr.is_empty() {
            // Try to write some bytes to the socket.
            //
            // Returns early if `poll_write` is not ready or errors.
            let n = try_ready!(self.socket.poll_write(&self.wr));

            // Asserts invariant that we always write something if `poll_write`
            // was ready.
            assert!(n > 0);

            // Discards the first `n` bytes of the buffer.
            let _ = self.wr.split_to(n);
        }

        Ok(Async::Ready(()))
    }
}

impl Stream for Lines {
    type Item = BytesMut;
    type Error = io::Error;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        // If the socket is ready, then it needs to be closed.
        let sock_closed = self.fill_read_buf()?.is_ready();

        // Searches for the CRLF character for a new line.
        //
        // Iterates over overlapping "windows" of two bytes.
        let pos = self.rd.windows(2).position(|bytes| bytes == b"\r\n");

        if let Some(pos) = pos {
            // Removes the line from read buffer, including "\r\n".
            let mut line = self.rd.split_to(pos + 2);

            // Removes the "\r\n" from `line`.
            line.split_off(pos);

            // Returns the line.
            return Ok(Async::Ready(Some(line)));
        }

        if sock_closed {
            Ok(Async::Ready(None))
        } else {
            // This only runs if underlying socket `read_buf` returned
            // NotReady.
            Ok(Async::NotReady)
        }
    }
}

/// Future that processes the broadcast logic for a connection.
struct Peer {
    /// Name of the peer. The first line recieved from the client.
    name: BytesMut,

    /// The TCP socket wrapped with the `Lines` codec.
    lines: Lines,

    /// Handle to the shared chat state.
    state: Arc<Mutex<Shared>>,

    /// Receive half of the message channel.
    ///
    /// This is used to recieve messages from peers. When a message is received
    /// off of this `Rx`, it will be written to the socket.
    rx: Rx,

    /// Client socket address.
    ///
    /// Used as the key to the `peers` HashMap stored in `state.
    addr: SocketAddr,
}

impl Peer {
    /// Creates a `Peer` instance.
    fn new(name: BytesMut, state: Arc<Mutex<Shared>>, lines: Lines) -> Peer {
        let addr = lines.socket.peer_addr().unwrap();

        // Create a channel for this peer.
        let (tx, rx) = mpsc::unbounded();

        // Adds an entry for this `Peer` to the shared state map.
        state.lock().unwrap().peers.insert(addr, tx);

        Peer {
            name,
            lines,
            state,
            rx,
            addr,
        }
    }
}

impl Drop for Peer {
    /// Removes the entry from the shared state map when it is dropped.
    fn drop(&mut self) {
        self.state.lock().unwrap().peers.remove(&self.addr);
    }
}

impl Future for Peer {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(), io::Error> {
        // Recieve all messages from peers.
        loop {
            // Pulls out all bytes from reciever.
            match self.rx.poll().unwrap() {
                Async::Ready(Some(v)) => {
                    // Buffer the line. Does this until no more lines are
                    // received from rx.
                    self.lines.buffer(&v);
                }
                _ => break,
            }
        }

        // Flush the write buffer to the socket.
        let _ = self.lines.poll_flush()?;

        // Read new lines from the socket
        while let Async::Ready(line) = self.lines.poll()? {
            println!("Recieved lines ({:?}) : {:?}", self.name, line);

            if let Some(message) = line {
                let mut line = self.name.clone();
                line.extend_from_slice(b": ");
                line.extend_from_slice(&message);
                line.extend_from_slice(b"\r\n");

                // Converts `line` to immutable, allowing zero copy cloning.
                let line = line.freeze();

                for (addr, tx) in &self.state.lock().unwrap().peers {
                    // Send to all other addresses that is not the peer's own.
                    if *addr != self.addr {
                        tx.unbounded_send(line.clone()).unwrap();
                    }
                }
            } else {
                // EOF was reached. The remote client disconnected.
                return Ok(Async::Ready(()));
            }
        }

        // Only return NotReady if either self.rx is NotReady, indicating that
        // it does not have any bytes recieved availiable and self.lines is
        // NotReady, indicating that there is no message to send out to other
        // peers.
        Ok(Async::NotReady)
    }
}

fn process(socket: TcpStream, state: Arc<Mutex<Shared>>) {
    let lines = Lines::new(socket);
    // Converts `lines` stream into a future which resolves into a pair
    // containing the next line and the remaining stream.
    let connection = lines
        .into_future()
        // StreamFuture's error type contains the stream itself, which we
        // don't need.
        .map_err(|(e, _)| e)
        // Process the first line recieved as the client's name.
        .and_then(|(name, lines)| {
            let name = match name {
                Some(name) => name,
                None => {
                    // TODO: Handle early disconnect
                    unimplemented!();
                }
            };

            println!("`{:?}` is joining the chat", name);

            Peer::new(name, state, lines)
        })
        // Tasks must have error type of `()`
        .map_err(|e| {
            println!("Connection error = {:?}", e);
        });
    tokio::spawn(connection);
}

fn main() {
    // Wraps an initial shared state into a mutual exclusion objection into a
    // thread-safe referenced-counted pointer.
    let state = Arc::new(Mutex::new(Shared::new()));

    let addr = "127.0.0.1:6142".parse().unwrap();
    let listener = TcpListener::bind(&addr).expect("unable to bind TCP listener");

    // Creates a future that will accept and process incoming connections.
    let server = listener
        .incoming()
        .for_each(move |socket| {
            process(socket, Arc::clone(&state));
            Ok(())
        })
        .map_err(|err| {
            // Prints error to STDOUT.
            println!("Accept error = {:?}", err);
        });
    println!("Server running on localhost:6142");
    tokio::run(server);
}
