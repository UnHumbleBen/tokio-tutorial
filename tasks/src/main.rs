//! An example that fetches the value from a URI using an HTTP get and caches
//! the result.
//!
//! Source: [https://tokio.rs/docs/going-deeper/tasks/](https://tokio.rs/docs/going-deeper/tasks/)

use futures::future;
// use std::io::Error;
use std::time::Duration;
use tokio::prelude::future::Either;
use tokio::prelude::*;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;
use tokio::timer::{Error, Interval, Timeout};

/// Get a URI from some remote cache.
fn cache_get(uri: &str) -> impl Future<Item = Option<String>, Error = Error> {
    future::ok(Some(String::from(uri) + " GET response from cache"))
}

fn cache_put(uri: &str, val: String) -> impl Future<Item = (), Error = Error> {
    future::ok(())
}

/// Do a full HTTP get to a remote URL
fn http_get(uri: &str) -> impl Future<Item = String, Error = Error> {
    future::ok(String::from(uri) + " GET response")
}

/// Does a HTTP get request to `url`. Returns a `Future` that caches the
/// response and returns that response.
fn fetch_and_cache(url: &str) -> impl Future<Item = String, Error = Error> {
    let url = url.to_string();

    let response = http_get(&url)
        .and_then(move |response| cache_put(&url, response.clone()).map(|_| response));

    response
}

fn main() {
    let url = "https://example.com";

    let response = cache_get(url).and_then(move |resp| match resp {
        Some(resp) => Either::A(future::ok(resp)),
        None => Either::B(fetch_and_cache(url)),
    });

    let task = Timeout::new(response, Duration::from_secs(20))
        .map(|res| println!("Response = {}", res))
        .map_err(|err| eprintln!("Error = {:?}", err));
    let rt = Runtime::new().unwrap();
    let my_executor = rt.executor();
    my_executor.spawn(task);

    // Instead of updating cache on a miss, update cache value on an interval.
    let url = "https://example_of_interval_update.com";
    let update_cache = Interval::new_interval(Duration::from_secs(60))
        .for_each(move |_| fetch_and_cache(url).map(|resp| println!("updated cache with {}", resp)))
        .map_err(|err| eprintln!("Error = {:?}", err));
    my_executor.spawn(update_cache);
    let response = cache_get(url);
    let task = Timeout::new(response, Duration::from_secs(20))
        .map(|res| {
            println!(
                "Response = {}\nbut this is impossible because cache has not been set yet!",
                res.unwrap()
            )
        })
        .map_err(|err| eprintln!("Error = {:?}", err));

    my_executor.spawn(task);

    // Demonstrate message passing to notify that first cache is ready.
    //
    //
    let url = "https://example_message_passing.com";

    let (primed_tx, primed_rx) = oneshot::channel();

    // Same as above `update_cache`, but there is now a `then` to wait for the
    // first successful cache to resolve, and then another `then` to wait for
    // message to be enqueued.
    let update_cache = fetch_and_cache(url)
        .map(|resp| println!("updated cache with {}", resp))
        .then(move |_| primed_tx.send(()))
        .then(move |_| {
            Interval::new_interval(Duration::from_secs(60)).for_each(move |_| {
                fetch_and_cache(url).map(|resp| println!("updated cache with {}", resp))
            })
        })
        .map_err(|err| eprintln!("Error = {:?}", err));
    my_executor.spawn(update_cache);

    // Same as `response` above, but waits for `primed_rx` to recieve a message
    // from `primed_tx`.
    let response = primed_rx.then(move |_| cache_get(url));

    let task = Timeout::new(response, Duration::from_secs(20))
        .map(|res| println!("Response = {}", res.unwrap()))
        .map_err(|err| eprintln!("Error = {:?}", err));

    my_executor.spawn(task);
    // shutdown_on_idle creates a Shutdown future.
    // `wait` blocks the thread until Shutdown is resolved.
    rt.shutdown_on_idle().wait().unwrap();
}
