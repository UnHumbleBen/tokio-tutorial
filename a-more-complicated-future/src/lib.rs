//! Implementing a future that takes a host name and establishes a connection
//! to the remote host.
//!
//! Source: [https://tokio.rs/docs/going-deeper/futures/](https://tokio.rs/docs/going-deeper/futures/)

use futures::try_ready;
use std::io;
use std::net::SocketAddr;
use tokio::net::tcp::ConnectFuture;
use tokio::net::TcpStream;
use tokio::prelude::*;

/// A future used to perform a DNS lookup.
///
/// Created by the `resolve` function.
///
/// # Note:
///
/// This is just a dummy implementation to simply the code. This future is
/// not the focus of this example. Focus on `ResolveAndConnect`.
pub struct ResolveFuture(String);

impl Future for ResolveFuture {
    type Item = SocketAddr;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        println!("Looking up host name = {}...", self.0);
        Ok(Async::Ready("127.0.0.1:12345".parse().unwrap()))
    }
}

/// Creates a future that looks up a host name `host`.
///
/// The returned future will return with the corresponding `SocketAddr`
/// if it exists.
///
/// # Note:
///
/// This is just a dummy implementation to simply the code. This future is
/// not the focus of this example. Focus on `ResolveAndConnect`.
pub fn resolve(host: &str) -> ResolveFuture {
    ResolveFuture(String::from(host))
}

/// Tracks the state of the future.
enum State {
    /// Currently resolving the host name.
    Resolving(ResolveFuture),
    /// Establishing a TCP connection to the remote host.
    Connecting(ConnectFuture),
}

/// A future used to take a host name, performs DNS resolution,
/// and establishes a connection to the remote host.
pub struct ResolveAndConnect {
    state: State,
}

/// Creates a future that looks up a host name `host`, performs DNS reslution,
/// and establishes a connection to the remote host.
///
/// The returned future will return a `TcpStream` connected to the
/// corresponding remote host.
pub fn resolve_and_connect(host: &str) -> ResolveAndConnect {
    let state = State::Resolving(resolve(host));
    ResolveAndConnect { state }
}

impl Future for ResolveAndConnect {
    type Item = TcpStream;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<TcpStream, io::Error> {
        use self::State::*;
        loop {
            let addr = match self.state {
                Resolving(ref mut fut) => try_ready!(fut.poll()),
                Connecting(ref mut fut) => {
                    return fut.poll();
                }
            };

            // State must be Resolving(ResolveFuture) and ResolvingFuture is
            // Ready.
            let connecting = TcpStream::connect(&addr);
            self.state = Connecting(connecting);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shorter_implementation() {
        let host = "www.example.com";
        let _resolve_and_connect = resolve(host).and_then(|addr| TcpStream::connect(&addr));
    }
}
