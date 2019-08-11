use tokio::io;
use tokio::net::TcpListener;

use futures::future::lazy;
use futures::{Future, Stream};

fn main() {
    let addr = "127.0.0.1:1234".parse().unwrap();
    let listener = TcpListener::bind(&addr).unwrap();

    tokio::run({
        listener
            .incoming()
            .take(10)
            .for_each(|socket| {
                tokio::spawn(lazy(|| {
                    io::write_all(socket, "hello world")
                        .map(|_| ())
                        .map_err(|e| println!("Socket error = {:?}", e))
                }));

                Ok(())
            })
            .map_err(|e| println!("Listener error = {:?}", e))
    });
}
