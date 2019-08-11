use futures::{stream, try_ready, Async, Future, Poll, Stream};
use std::fmt::Display;
use std::time::Duration;
use tokio::timer::Interval;

/// A future that displays 10 items from a Stream of type `T`.
pub struct Display10Manual<T> {
    stream: T,
    curr: usize,
}

impl<T> Display10Manual<T> {
    /// Initializes a `Display10` that will display 10 items from `stream`.
    pub fn new(stream: T) -> Display10Manual<T> {
        Display10Manual { stream, curr: 0 }
    }
}

impl<T> Future for Display10Manual<T>
where
    T: Stream,
    T::Item: Display,
{
    type Item = ();
    type Error = T::Error;

    fn poll(&mut self) -> Poll<(), Self::Error> {
        while self.curr < 10 {
            let value = match try_ready!(self.stream.poll()) {
                Some(value) => value,
                None => break,
            };
            println!("Value #{} = {}", self.curr, value);
            self.curr += 1;
        }

        Ok(Async::Ready(()))
    }
}

/// Computes the fibonacci sequence.
pub struct FibonacciManual {
    interval: Interval,
    curr: u64,
    next: u64,
}

impl FibonacciManual {
    /// Initializes `Fibonacci`.
    pub fn new(duration: Duration) -> FibonacciManual {
        FibonacciManual {
            interval: Interval::new_interval(duration),
            curr: 1,
            next: 1,
        }
    }
}

impl Stream for FibonacciManual {
    type Item = u64;

    type Error = ();

    fn poll(&mut self) -> Poll<Option<u64>, ()> {
        try_ready!(self.interval.poll().map_err(|_| ()));

        let curr = self.curr;
        let next = curr + self.next;

        self.curr = self.next;
        self.next = next;

        Ok(Async::Ready(Some(curr)))
    }
}

fn fibonacci() -> impl Stream<Item = u64, Error = ()> {
    stream::unfold((1, 1), |(curr, next)| {
        let new_next = curr + next;
        Some(Ok((curr, (next, new_next))))
    })
}

fn main() {
    let fib = FibonacciManual::new(Duration::from_secs(1));
    let display = Display10Manual::new(fib);

    println!("Running manually implemented fibonacci ...");
    tokio::run(display);

    println!("Running combinator implemented fibonacci ...");
    tokio::run(fibonacci().take(10).for_each(|num| {
        println!("{}", num);
        Ok(())
    }));

    struct FibonacciCombinatorWithInterval {
        curr: u64,
        next: u64,
    }

    let mut fib = FibonacciCombinatorWithInterval { curr: 1, next: 1 };

    let future = Interval::new_interval(Duration::from_secs(1)).map(move |_| {
        let curr = fib.curr;
        let next = curr + fib.next;

        fib.curr = fib.next;
        fib.next = next;

        curr
    });

    println!("Running combinator implemented fibonacci with intervals ...");
    tokio::run(future.take(10).map_err(|_| ()).for_each(|num| {
        println!("{}", num);
        Ok(())
    }));

    let values = vec![
        Ok("one"),
        Err("something went wrong!"),
        Ok("three"),
        Ok("four"),
    ];

    println!("Running concrete stream converted from iterator ...");
    tokio::run(
        stream::iter_result(values)
            .map_err(|e| println!("{}", e))
            .for_each(|value| {
                println!("{}", value);
                Ok(())
            }),
    )
}
