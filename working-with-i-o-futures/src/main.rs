use tokio::net::TcpStream;
use tokio::prelude::*;

// It may be easier to run only one of the futures here, rather than all at
// once. Comment out two of the `tokio::run` lines, and leave the remaining one
// to try it ou.

fn main() {
    let addr = "127.0.0.1:12345".parse().unwrap();

    // Demonstrates read_exact.
    let read_8_fut = TcpStream::connect(&addr)
        .and_then(|stream| {
            let buf = vec![0; 8];
            tokio::io::read_exact(stream, buf)
            // Item = (TcpStream, Vec<u8>)
            // Error = io::Error
        })
        .inspect(|(_stream, buf)| {
            // https://doc.rust-lang.org/std/fmt/index.html#formatting-traits
            // x? ⇒ Debug with lower-case hexadecimal integers
            println!("got eight bytes: {:x?}", buf);
            // Item = (TcpStream, Vec<u8>)
            // Error = io::Error
        })
        .map(|(stream, buf)| {
            println!(
                "stream = {:?}\nbuff_as_string = {:x?}\n",
                stream,
                String::from_utf8(buf)
            );
        })
        .map_err(|error| {
            println!("error = {:?}", error);
        });
    tokio::run(read_8_fut);

    // Demonstrates write_all.
    let addr = "127.0.0.1:12346".parse().unwrap();
    let echo_fut = TcpStream::connect(&addr)
        .and_then(|stream| {
            let buf = vec![0; 32];
            tokio::io::read_exact(stream, buf)
        })
        .and_then(|(stream, buf)| {
            println!("This ran!");
            tokio::io::write_all(stream, buf)
        })
        .inspect(|(_stream, buf)| {
            println!("Echoed back {} bytes: {:x?}", buf.len(), buf);
        })
        .map(|(stream, buf)| {
            println!(
                "stream = {:?}\nbuff_as_string = {:x?}\n",
                stream,
                String::from_utf8(buf)
            );
        })
        .map_err(|error| {
            println!("error = {:?}", error);
        });
    tokio::run(echo_fut);

    // Demonstrates copy
    let addr = "127.0.0.1:12347".parse().unwrap();
    let echo_fut = TcpStream::connect(&addr)
        .and_then(|stream| {
            let (reader, writer) = stream.split();
            tokio::io::copy(reader, writer)
        })
        .inspect(|(bytes_copied, _, _)| {
            println!("echoed back {} bytes", bytes_copied);
        })
        .map(|(_bytes_copied, reader, writer)| {
            println!("reader = {:?}\nwriter = {:x?}\n", reader, writer);
        })
        .map_err(|error| {
            println!("error = {:?}", error);
        });
    tokio::run(echo_fut);

    // Demonstrates line
    let addr = "127.0.0.1:12348".parse().unwrap();
    let lines_fut = TcpStream::connect(&addr)
        .and_then(|stream| {
            // Adds BufRead trait.
            let stream = std::io::BufReader::new(stream);
            // BufRead required for `lines` function.
            tokio::io::lines(stream).for_each(|line| {
                println!("server sent us the line: {}", line);
                Ok(())
            })
        })
        .map_err(|error| {
            println!("error = {:?}", error);
        });
    tokio::run(lines_fut);
}
