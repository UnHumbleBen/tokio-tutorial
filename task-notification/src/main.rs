//! Implementing a server future that processes all inbounds connections on the
//! same socket.
//!
//! Notice that this implementation involves one big task. Whenever this task
//! is notified, it needs to iterate through every `connection` to see which
//! one can be removed.
//!
//! This is why servers spawn new tasks for each connection instead of
//! processing the connections in the same task as the listener.
//!
//! In general, try to keep tasks small.
//!
//! Source: [https://tokio.rs/docs/going-deeper/tasks/#](https://tokio.rs/docs/going-deeper/tasks/#)

use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;

fn process(socket: TcpStream) -> impl Future<Item = (), Error = io::Error> {
    let (reader, writer) = socket.split();
    io::copy(reader, writer).map(|_| ())
}
pub struct Server {
    listener: TcpListener,
    connections: Vec<Box<Future<Item = (), Error = io::Error> + Send>>,
}

impl Server {
    pub fn new(addr: &str) -> Server {
        let addr = addr.parse().unwrap();
        let listener = TcpListener::bind(&addr).unwrap();

        Server {
            listener,
            connections: vec![],
        }
    }
}

impl Future for Server {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Result<Async<()>, io::Error> {
        loop {
            match self.listener.poll_accept()? {
                Async::Ready((socket, _)) => {
                    let connection = process(socket);
                    self.connections.push(Box::new(connection));
                }
                Async::NotReady => break,
            }
        }

        let len = self.connections.len();

        // Iterate backwards to avoid skipping during deletion.
        for i in (0..len).rev() {
            match self.connections[i].poll()? {
                Async::Ready(_) => {
                    self.connections.remove(i);
                }
                Async::NotReady => {}
            }
        }

        Ok(Async::NotReady)
    }
}

fn main() {
    let addr = "127.0.0.1:12345";
    let server = Server::new(addr).map_err(|e| eprintln!("Error = {}", e));
    println!("Server running!\nStart up a listner with:\nnc localhost 12345");
    tokio::run(server);
}
