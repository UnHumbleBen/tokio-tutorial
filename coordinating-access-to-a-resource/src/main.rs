use futures::future::lazy;
use futures::sync::{mpsc, oneshot};
use futures::{future, Future, Sink, Stream};
use std::time::{Duration, Instant};
use tokio::io;

type Message = (oneshot::Sender<Duration>, u32);

#[derive(Debug)]
struct Transport;

impl Transport {
    fn send_ping(&self, i: u32) {
        println!("Sending ping #{}...\n", i);
        std::thread::sleep(Duration::from_secs(1));
    }

    fn recv_pong(&self, i: u32) -> impl Future<Item = (), Error = io::Error> {
        println!("Recieving pong #{}...\n", i);
        std::thread::sleep(Duration::from_secs(1));
        future::ok(())
    }
}

fn coordinator_task(rx: mpsc::Receiver<Message>) -> impl Future<Item = (), Error = ()> {
    println!("Initializing Transport...\n");

    let transport = Transport;
    println!("{:#?}", Transport);

    rx.for_each(move |pong_tx| {
        println!("----response transmiter #{} recieved by rx!\n", pong_tx.1);
        let start = Instant::now();

        transport.send_ping(pong_tx.1);

        transport
            .recv_pong(pong_tx.1)
            .map_err(|_| ())
            .and_then(move |_| {
                let rtt = start.elapsed();
                println!("-------- Duration #{} sent by rx!\n", pong_tx.1);
                pong_tx.0.send(rtt).unwrap();
                Ok(())
            })
    })
}

fn rtt(
    tx: mpsc::Sender<Message>,
    i: u32,
) -> impl Future<Item = (Duration, mpsc::Sender<Message>), Error = ()> {
    let (resp_tx, resp_rx) = oneshot::channel();

    tx.send((resp_tx, i)).map_err(|_| ()).and_then(move |tx| {
        println!("----response transmiter #{} sent by tx!\n", i);
        resp_rx.map(|dur| (dur, tx)).map_err(|_| ())
    })
}

fn main() {
    println!("Starting tokio runtime...\n");
    tokio::run(lazy(|| {
        println!("Creating channel...\n");
        let (tx, rx): (mpsc::Sender<Message>, _) = mpsc::channel(1_024);
        println!("tx = {:#?}\nrx = {:#?}\n", tx, rx);

        println!("Spawning coordinator task...\n");
        tokio::spawn(coordinator_task(rx));

        for i in 0..1 {
            println!("Cloning transmiter #{}...\n", i);
            let tx = tx.clone();
            println!("tx = {:#?}\n", tx);

            println!("Spawning requests for round trip time...\n");
            tokio::spawn({
                rtt(tx, i).and_then(move |(dur, _)| {
                    println!("-------- Duration #{} recieved by tx!\n", i);
                    println!("Duration #{} = {:?}\n", i, dur);
                    Ok(())
                })
            });
        }

        println!(" ---------------------------- ");
        println!("|    Spawning completed      |");
        println!(" ---------------------------- \n");

        Ok(())
    }));
}
