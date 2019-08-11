use tokio::net::TcpListener;
use tokio::prelude::*;

fn main() {
    let addr = "127.0.0.1:12345".parse().unwrap();
    let listener = TcpListener::bind(&addr).expect("unable to bind TCP listener");
    let incoming = listener.incoming();
    let server = incoming
        .map_err(|e| eprintln!("accept failed = {:?}", e))
        .for_each(|socket| {
            let (reader, writer) = socket.split();
            let bytes_copied = tokio::io::copy(reader, writer);
            let handle_conn = bytes_copied
                .map(|amt| println!("Wrote {:?} bytes", amt.0))
                .map_err(|err| eprintln!("I/O error {:?}", err));

            tokio::spawn(handle_conn)
        });
    tokio::run(server);
}
