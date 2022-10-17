use std::{fs, net::SocketAddr, time::Duration};

use serde_derive::Deserialize;
use toml;

use ratelimit_rs::StreamHandler;

use async_std::io;
use async_std::net::TcpListener;
use async_std::sync::Arc;
use async_std::task;

use futures::lock::Mutex;
use futures::stream::StreamExt;

use ratelimit_rs::{Ratelimit, RatelimitCollection};

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

#[derive(Deserialize, Debug)]
struct RLConfig {
    hits: u32,
    seconds: u32,

    cleanup_interval: u32,
}

#[derive(Deserialize, Debug)]
struct MCacheConfig {
    listen: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct HandlersConfig {
    memcache: MCacheConfig,
}

#[derive(Deserialize, Debug)]
struct Config {
    ratelimit: RLConfig,
    handlers: HandlersConfig,
}

fn main() -> io::Result<()> {
    let conf_str = fs::read_to_string("development.toml")?;
    let config: Config = toml::from_str(&conf_str)?;

    let ratelimit = Ratelimit::new(config.ratelimit.hits, config.ratelimit.seconds * 1000).unwrap();

    let arc = Arc::new(Mutex::new(ratelimit));
    let arc_collection = Arc::new(Mutex::new(RatelimitCollection::new()));

    let addresses: Vec<SocketAddr> = config
        .handlers
        .memcache
        .listen
        .clone()
        .iter()
        .map(|x| x.parse().unwrap())
        .collect();

    if addresses.len() == 0 {
        return Ok(()); // Not ok
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
