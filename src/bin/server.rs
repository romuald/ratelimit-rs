use std::process::exit;
use std::{net::SocketAddr, time::Duration};

use ratelimit_rs::StreamHandler;

use async_std::io;
use async_std::net::TcpListener;
use async_std::sync::Arc;
use async_std::task;

use futures::lock::Mutex;
use futures::stream::StreamExt;

use ratelimit_rs::{Configuration, Ratelimit, RatelimitCollection};

async fn cleanup_timer(
    duration: Duration,
    rl_arc: Arc<Mutex<Ratelimit>>,
    meta_arc: Arc<Mutex<RatelimitCollection>>,
) {
    //let dur = Duration::from_millis(CLEANUP_INTERVAL);

    loop {
        task::sleep(duration).await;

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
    let config = Configuration::from_argv()?;

    let ratelimit = Ratelimit::new(
        config.ratelimit.hits,
        (config.ratelimit.seconds * 1000f64) as u32,
    )
    .unwrap();

    let arc = Arc::new(Mutex::new(ratelimit));
    let arc_collection = Arc::new(Mutex::new(RatelimitCollection::default()));

    let memcache_config = config.handlers.memcache;

    let addresses: Vec<SocketAddr> = memcache_config
        .listen
        .iter()
        .map(|x| x.parse().unwrap())
        .collect();

    if !memcache_config.enabled {
        eprintln!("No server is enabled");
        exit(1);
    }
    if addresses.is_empty() {
        eprintln!("No listen addresses configured for memcache server");
        exit(1);
    }

    let cleanup_duration = Duration::from_secs(config.ratelimit.cleanup_interval as u64);
    task::spawn(cleanup_timer(
        cleanup_duration,
        arc.clone(),
        arc_collection.clone(),
    ));

    task::block_on(async {
        let listener = TcpListener::bind(&addresses[..]).await?;
        let mut incoming = listener.incoming();

        while let Some(stream) = incoming.next().await {
            let mut stream = stream?;
            let mut handler = StreamHandler::new(&arc, &arc_collection);
            task::spawn(async move { handler.main(&mut stream).await });
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
