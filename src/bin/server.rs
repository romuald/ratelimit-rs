use std::str;
use std::time::Duration;

use lazy_static::lazy_static;

use regex::Regex;

use async_std::io;
use async_std::net::TcpListener;
use async_std::prelude::*;
use async_std::task;
use async_std::sync::Arc;

use futures::lock::Mutex;
use futures::stream::StreamExt;

use std::time::Instant;
use ratelimit_rs::{Ratelimit, MetaRatelimit};

const HITS: u32 = 5;
const DURATION_MS: u32 = 10_000;
const CLEANUP_INTERVAL: u64 = 10_000;

enum Command {
    INCR,
}

fn read_input(buffer: [u8; 512], len: usize) -> Result<(Command, String), ()> {
    let input = match str::from_utf8(&buffer[0..len]) {
        Ok(v) => v,
        Err(_) => return Err(())
    }.trim();

    let (command, key) = {
        let mut split = input.split(" ");
        (
        match split.next() {
                Some(x) => x.trim(),
                None => return Err(()),
            },
        match split.next() {
            Some(x) => x.trim(),
            None => return Err(()),
        })
    };

    match command {
        "incr" => Ok((Command::INCR, String::from(key))),
        _ => Err(())
    }
}

// "1/2_foo" => (1, 2_000, "foo")
fn parse_specification(keyname: &String) -> Option<(u32, u32, String)> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^(\d+)/(\d+)_(.+)").unwrap();
    }

    let m = RE.captures(&keyname);
    let caps = m?;
    let hits = caps.get(1)?.as_str().to_string();
    let seconds = caps.get(2)?.as_str().to_string();
    let keyname = caps.get(3)?.as_str().to_string();

    let hits = hits.parse();
    let seconds = seconds.parse::<u32>();

    if hits.is_ok() && seconds.is_ok() {
        Some((hits.unwrap(), seconds.unwrap() * 1000, keyname))
    } else {
        None
    }
}


#[test]
fn test_parse_specification() {
    assert_eq!(parse_specification(&"toto".to_string()), None);
    assert_eq!(parse_specification(&"1zzb_zo".to_string()), None);
    assert_eq!(parse_specification(&"1/2_toto".to_string()), Some((1, 2000, "toto".to_string())));
    assert_eq!(parse_specification(&"80/200_bar".to_string()), Some((80, 200_000, "bar".to_string())));
    assert_eq!(parse_specification(&"1/999999999999999_toto".to_string()), None);
    assert_eq!(parse_specification(&"99999999999999/99_toto".to_string()), None);
}

fn main() -> io::Result<()> {
    task::block_on(async {
        let listener = TcpListener::bind("127.0.0.1:11211").await?;

        let ratelimit_arc =  Arc::new(Mutex::new(Ratelimit::new(HITS, DURATION_MS)));
        let ratelimit_arc_main = ratelimit_arc.clone();
        let ratelimit_arc_cleanup = ratelimit_arc.clone();

        let meta_ratelimit_arc =  Arc::new(Mutex::new(MetaRatelimit::new()));
        let meta_ratelimit_arc_main = meta_ratelimit_arc.clone();
        let meta_ratelimit_arc_cleanup = meta_ratelimit_arc.clone();
        
        task::spawn(async move {
            let arc = ratelimit_arc_cleanup.clone();
            let meta_arc = meta_ratelimit_arc_cleanup.clone();
            let dur = Duration::from_millis(CLEANUP_INTERVAL);

            loop {
                task::sleep(dur).await;

                let start = Instant::now();
                let c=  {
                    let mut ratelimit = arc.lock().await;
                    ratelimit.cleanup()
                } + {
                    let mut meta = meta_arc.lock().await;
                    meta.cleanup()
                };
                // Warning: lock + .await means the elapsed time may not be correct
                let end = Instant::now();
                println!("cleanup time: {:?} ({:?} removed)", (end - start), c);
            }
        });

        let mut incoming = listener.incoming();
        while let Some(stream) = incoming.next().await {
            let mut stream = stream?;
            let arc = ratelimit_arc_main.clone();
            let meta_arc = meta_ratelimit_arc_main.clone();

            task::spawn(async move {
                let mut buffer = [0; 512];

                loop {
                    // Read from TCP stream, up to n bytes
                    let read = match stream.read(&mut buffer).await {
                        Ok(n) => n,
                        Err(_) => break,
                    };

                    // Empty read: the remote side closed the connection
                    if read == 0 {
                        break;
                    }

                    let response = match read_input(buffer, read) {
                        // Bad input
                        Err(()) => "ERR\r\n",
                        Ok((command, key)) => {
                            match command {
                                Command::INCR => {
                                    let spec = parse_specification(&key);

                                    match spec {
                                        Some((hits, duration, keyname)) => {
                                            let mut meta = meta_arc.lock().await;
                                            if meta.hit(hits, duration, keyname) {
                                                "0\r\n"
                                            } else {
                                                "1\r\n"
                                            }
                                        },
                                        None => {
                                            let mut ratelimit = arc.lock().await;
                                            if ratelimit.hit(&key.to_string()) {
                                                "0\r\n"
                                            } else {
                                                "1\r\n"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    };

                    if stream.write(response.as_bytes() ).await.is_err() || stream.flush().await.is_err() {
                        break;
                    }
                }
            });
            
        }
        Ok(())
    })
}
