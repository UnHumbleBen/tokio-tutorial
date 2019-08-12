use tokio::prelude::*;
use tokio::timer::Interval;

use std::time::Duration;

fn main() {
    // Takes 10 seconds total, first fires happens at 1 second, not
    // immediately.
    let task = Interval::new_interval(Duration::from_millis(1000))
        .take(10)
        .for_each(|instant| {
            println!("fire; instant={:?}", instant);
            Ok(())
        })
        .map_err(|e| panic!("interval errored; err={:?}", e));
    tokio::run(task);
}
