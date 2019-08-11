use futures::future::lazy;

fn main() {
    tokio::run(lazy(|| {
        for i in 0..4 {
            tokio::spawn(lazy(move || {
                println!("Hello from task {}", i);
                Ok(())
            }));
        }

        Ok(())
    }));
}
