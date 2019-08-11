use futures::sync::mpsc;
use tokio::io;
use tokio::net::TcpListener;
use tokio::timer::Interval;

use futures::future::lazy;
use futures::{future, stream, Future, Sink, Stream};

use std::time::Duration;

// Defines the background task. The `rx` argument is the channel receive
// handle. The task will pull `usize` values (which represent number of bytes
// ready by a socket) off the channel and sum it internally. Every 30 seconds,
// the current sum is written to STDOUT and the sum is reset to zero.
fn bg_task(rx: mpsc::Receiver<usize>) -> impl Future<Item = (), Error = ()> {
    println!("Running bg_task!");
    // The stream of received `usize` values will be merge with a 30 second
    // interval stream. The value types of each stream must match. This enum is
    // used to track the various values.
    #[derive(Eq, PartialEq)]
    enum Item {
        Value(usize),
        Tick,
        Done,
    }

    // Interval at which the current sum is written to STDOUT.
    let tick_dur = Duration::from_secs(10);

    let interval = Interval::new_interval(tick_dur)
        .map(|_| {
            println!("----Item::Tick");
            Item::Tick
        })
        .map_err(|_| ());

    // Turn the stream into a sequence of:
    // Item(num), Item(num), ... Done
    //
    rx.map(|len| {
        println!("----Item::Value({})", len);
        Item::Value(len)
    })
    .chain(stream::once(Ok(Item::Done)))
    .map(|item| match item {
        Item::Done => {
            println!("----Item::Done");
            item
        }
        _ => item,
    })
    // Merge in the stream of intervals
    .select(interval)
    // Terminate the stream once `Done` is received. This is necessary
    // because `Interval` is an infinite stream and `select` will keep
    // selecting on it.
    .take_while(|item| {
        future::ok(*item != Item::Done).map(|is_done| {
            println!("----TakeWhile poll");
            is_done
        })
    })
    // With the stream of `Item` values, start our logic.
    //
    // Using `fold` allows the state to be maintained across iterations.
    // In this case, the state is the number of read bytes between tick.
    .fold(0, |num, item| {
        println!("----Fold poll");
        match item {
            // Sum the number of bytes with the state.
            Item::Value(v) => {
                // println!("Adding {}", v);
                println!("---------------------------");
                future::ok(num + v)
            }
            Item::Tick => {
                println!("Bytes read = {}", num);

                println!("---------------------------");
                // Reset the byte counter.
                future::ok(0)
            }
            _ => unreachable!(),
        }
    })
    .map(|_| ())
}

fn main() {
    // Start the application.
    tokio::run(lazy(|| {
        println!("Running closure!");
        let addr = "127.0.0.1:1234".parse().unwrap();
        let listener = TcpListener::bind(&addr).unwrap();

        // Creates the channel that is used to communicate with the background
        // task.
        let (tx, rx) = mpsc::channel(1_024);

        // Spawn the background task.
        tokio::spawn(bg_task(rx));

        listener
            .incoming()
            .take(10)
            .for_each(move |socket| {
                // println!("Incoming socket!");
                // An inbound socket has been received.
                //
                // Spawn a new task to process the socket.
                tokio::spawn({
                    // Clones the sender handle.
                    let tx = tx.clone();

                    // All bytes read from the socket will be placed into a
                    // `vec`.
                    io::read_to_end(socket, vec![])
                        // Drops the socket.
                        .and_then(|(_, buf)| {
                            // println!("Sending message!");
                            tx.send(buf.len()).map_err(|_| io::ErrorKind::Other.into())
                        })
                        .map(|_| ())
                        // Writes any error to STDOUT
                        .map_err(|e| println!("Socket error = {:?}", e))
                });

                // println!("Dropping socket!");

                // Recieve the next inbound socket.
                Ok(())
            })
            .map_err(|e| println!("Listener error = {:?}", e))
    }));
}
