//! Example usage of the poll model to retrieve a widget.
//!
//! Source: [https://tokio.rs/docs/going-deeper/runtime-model/](https://tokio.rs/docs/going-deeper/runtime-model/)
//!
//! Example implementation of an executor.

use std::collections::VecDeque;
use tokio::prelude::*;

/// A dummy Widget struct.
#[derive(Debug)]
struct Widget;

/// Returns an `Async<Widget>` where `Async` is an enum of `Ready(Widget)` or
/// `NotReady`. This `Async` enum is provided by the futures crate and is one
/// of the building blocks of the polling model.
fn poll_widget() -> Async<Widget> {
    Async::Ready(Widget)
}

/// A task that polls a single widget and writes it to STDOUT.
pub struct MyTask;

impl Future for MyTask {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Result<Async<()>, ()> {
        match poll_widget() {
            Async::Ready(widget) => {
                println!("widget = {:?}", widget);
                Ok(Async::Ready(()))
            }
            Async::NotReady => Ok(Async::NotReady),
        }
    }
}

pub struct SpinExecutor {
    // Double ended queue containing the tasks the executor is responsible for.
    // Used in inefficient version.
    tasks: VecDeque<Box<Future<Item = (), Error = ()> + Send>>,

    // Fields for more efficient implementation.
    ready_tasks: VecDeque<Box<Future<Item = (), Error = ()> + Send>>,
    not_ready_tasks: VecDeque<Box<Future<Item = (), Error = ()> + Send>>,
}

impl SpinExecutor {
    pub fn new() -> SpinExecutor {
        SpinExecutor {
            tasks: VecDeque::new(),
            ready_tasks: VecDeque::new(),
            not_ready_tasks: VecDeque::new(),
        }
    }
    pub fn spawn<T>(&mut self, task: T)
    where
        T: Future<Item = (), Error = ()> + 'static + Send,
    {
        self.tasks.push_back(Box::new(task));
    }

    /// Runs all the tasks assigned to this `SpinExecutor.
    ///
    /// Not very efficient because it continuously polls tasks may still may
    /// not be ready yet.
    pub fn run_inefficient(&mut self) {
        while let Some(mut task) = self.tasks.pop_front() {
            match task.poll().unwrap() {
                Async::Ready(_) => {}
                Async::NotReady => {
                    // If the task is not ready, push it to the back of the
                    // queue.
                    self.tasks.push_back(task);
                }
            }
        }
    }

    /// Ideal implementation of `run`, relies on some notifiers.
    pub fn run(&mut self) {
        loop {
            while let Some(mut task) = self.ready_tasks.pop_front() {
                match task.poll().unwrap() {
                    Async::Ready(_) => {}
                    Async::NotReady => {
                        self.not_ready_tasks.push_back(task);
                    }
                }
            }

            if self.not_ready_tasks.is_empty() {
                return;
            }

            // Puts the thread until there is work to do.
            self.sleep_until_tasks_are_ready();
        }
    }

    /// Ideally this function will stop `run` until a new task goes from "not
    /// ready" to "ready".
    pub fn sleep_until_tasks_are_ready(&mut self) {}
}

fn main() {
    tokio::run(MyTask);
    let mut my_executor = SpinExecutor::new();
    my_executor.spawn(MyTask);
    my_executor.spawn(MyTask);
    my_executor.spawn(MyTask);
    my_executor.spawn(MyTask);
    my_executor.spawn(MyTask);
    my_executor.spawn(MyTask);
    my_executor.spawn(MyTask);
    my_executor.run_inefficient();
}
