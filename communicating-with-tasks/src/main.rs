use futures::future::lazy;
use futures::sync::mpsc;
use futures::sync::oneshot;
use futures::{stream, Future, Sink, Stream};

fn main() {
    // mpsc
    tokio::run(lazy(|| {
        let (tx, rx) = mpsc::channel(1_024);

        tokio::spawn({
            stream::iter_ok(0..10)
                .fold(tx, |tx, i| {
                    tx.send(format!("Message {} from spawned task", i))
                        .map_err(|e| println!("error = {:?}", e))
                })
                .map(|_| ())
        });

        rx.for_each(|msg| {
            println!("Got `{}`", msg);
            Ok(())
        })
    }));
    // oneshot
    tokio::run(lazy(|| {
        let (tx, rx) = oneshot::channel();

        tokio::spawn(lazy(|| {
            tx.send("hello from spawned task").unwrap();
            Ok(())
        }));

        rx.and_then(|msg| {
            println!("Got `{}`", msg);
            Ok(())
        })
        .map_err(|e| println!("error = {:?}", e))
    }));
}
