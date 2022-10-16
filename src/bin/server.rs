use std::time::Duration;

use ratelimit_rs::StreamHandler;

use async_std::io;
use async_std::net::TcpListener;
use async_std::sync::Arc;
use async_std::task;

use futures::lock::Mutex;
use futures::stream::StreamExt;

// use std::time::Instant;
use ratelimit_rs::{Ratelimit, RatelimitCollection};

const HITS: u32 = 5;
const DURATION_MS: u32 = 10_000;
const CLEANUP_INTERVAL: u64 = 10_000;

async fn cleanup_timer(rl_arc: Arc<Mutex<Ratelimit>>, meta_arc: Arc<Mutex<RatelimitCollection>>) {
    //println!("here?");
    let dur = Duration::from_millis(CLEANUP_INTERVAL);

    loop {
        task::sleep(dur).await;

        // let start = Instant::now();
        let _c = {
            let mut ratelimit = rl_arc.lock().await;
            ratelimit.cleanup()
        } + {
            let mut meta = meta_arc.lock().await;
            meta.cleanup()
        };
        // Warning: lock + .await means the elapsed time may not be correct
        // let end = Instant::now();
        // println!("cleanup time: {:?} ({:?} removed)", (end - start), c);
    }
}

fn main() -> io::Result<()> {
    task::block_on(async {
        let listener = TcpListener::bind("127.0.0.1:11211").await?;

        let arc = Arc::new(Mutex::new(Ratelimit::new(HITS, DURATION_MS).unwrap()));
        let arc_collection = Arc::new(Mutex::new(RatelimitCollection::new()));

        task::spawn(cleanup_timer(arc.clone(), arc_collection.clone()));

        let mut incoming = listener.incoming();
        while let Some(stream) = incoming.next().await {
            let mut handler = StreamHandler::new(stream?, &arc, &arc_collection);
            task::spawn(async move { handler.main().await });
        }
        Ok(())
    })
}

#[cfg(test)]
mod test {
    #[async_std::test]
    async fn test_xx() {
        assert_eq!(1, 1)
    }
}
