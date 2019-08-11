# tokio-tutorial

An collection of annotated code examples from the
[Tokio](https://tokio.rs/) documentation.

## Running Examples

Examples can be run with

```
cargo run --bin [example-project-name]
```

Some examples involve sending and recieving bytes
from a TCP socket, following the Client/Server
Model. Refer to [netcat's man page](https://linux.die.net/man/1/nc)

When the example code creates a listener (with 
`TcpListener`), you can make a client by

```
nc localhost [port number]
```

When the example code attempts to connect to an
address (with `TcpStream`), you can make a
listener by

```
nc -l -p [port number]
```

## Common Errors

```
error = Connection refused (os error 111)
```

Likely cause: Lack of listener. Start one up with netcat and run
the example.

## tokio-0.1

Keep in mind that the documentation (and hence the example code)
use tokio-0.1, so some features may be depreciated by the time
tokio-0.2 is stabilized.
