use tokio::io;
use tokio::net::TcpStream;
use tokio::prelude::*;

use std::time::{Duration, Instant};

fn read_four_bytes(
    socket: TcpStream,
) -> Box<Future<Item = (TcpStream, Vec<u8>), Error = ()> + Send> {
    let buf = vec![0; 4];
    let fut = io::read_exact(socket, buf)
        .timeout(Duration::from_secs(5))
        .map_err(|_| println!("failed to read 4 bytes by timeout"));
    Box::new(fut)
}
fn main() {
    let addr = "127.0.0.1:12345".parse().unwrap();
    let stream = TcpStream::connect(&addr).wait().unwrap();
    let fut = read_four_bytes(stream).map(|_| ());
    tokio::run(fut)
}
