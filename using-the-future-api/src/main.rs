use tokio::io;
use tokio::net::TcpListener;
use tokio::prelude::*;

fn main() {
    let addr = "127.0.0.1:12345".parse().unwrap();
    let listener = TcpListener::bind(&addr).expect("unable to bind TCP listener");
    let server = listener
        .incoming()
        .for_each(|socket| {
            println!("accepted socket; addr={:?}", socket.peer_addr().unwrap());

            let buf = vec![0; 5];

            let connection = io::read_exact(socket, buf)
                .and_then(|(socket, buf)| io::write_all(socket, buf))
                .then(|_| Ok(())); // Just discard the socket and buffer.
            tokio::spawn(connection);

            Ok(())
        })
        .map_err(|e| eprintln!("Error = {:?}", e)); // Discard error too.

    tokio::run(server);
}
