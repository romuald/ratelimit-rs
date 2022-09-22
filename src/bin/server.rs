
use std::str;

use std::time::Duration;

use async_std::net::TcpStream;
use lazy_static::lazy_static;

use regex::Regex;

use async_std::io;
use async_std::net::TcpListener;
use async_std::prelude::*;
use async_std::task;
use async_std::sync::Arc;

use futures::lock::Mutex;
use futures::stream::StreamExt;

// use std::time::Instant;
use ratelimit_rs::{Ratelimit, RatelimitCollection};

const HITS: u32 = 5;
const DURATION_MS: u32 = 10_000;
const CLEANUP_INTERVAL: u64 = 10_000;

struct StreamHandler {
    stream: TcpStream,
    ratelimit: Arc<Mutex<Ratelimit>>,
    ratelimit_collection: Arc<Mutex<RatelimitCollection>>,
}

/// StreamHandler
/// Handles a single TCP stream
impl StreamHandler {
    pub fn new(stream: TcpStream, ratelimit: &Arc<Mutex<Ratelimit>>, ratelimit_collection: &Arc<Mutex<RatelimitCollection>> ) -> StreamHandler {
        StreamHandler {
            stream: stream,
            ratelimit: ratelimit.clone(),
            ratelimit_collection: ratelimit_collection.clone(),
        }
    }

    /// An "OK" response, request was within the limits (ironically 0)
    async fn reply_ok(&mut self) -> bool {
        self.write("0\r\n").await
    }

    /// An "not OK" response, request was outside the limits and should be limited (ironically 0)
    async fn reply_ko(&mut self) -> bool {
        self.write("1\r\n").await
    }

    /// An "error" response, the request was malformed or using a bad syntax
    async fn reply_err(&mut self) -> bool {
        self.write("ERR\r\n").await
    }

    /// Flush the binary response
    async fn write(&mut self, response: &str) -> bool {
        self.stream.write(response.as_bytes()).await.is_ok() && self.stream.flush().await.is_ok()
    }

    /// Handles an "incr" command
    /// Will write the response on the output stream
    /// Can return an error in case the keyname is invalid
    async fn handle_incr(&mut self, keyname: &str) -> Result<(), Box<dyn std::error::Error>>{
        let within_limits = match parse_specification(&keyname) {
            Some((hits, duration, keyname)) => {
                let mut meta = self.ratelimit_collection.lock().await;
                let rl = meta.get_instance(hits, duration)?;
                rl.hit(&keyname)
            },
            None => {
                let mut ratelimit = self.ratelimit.lock().await;
                ratelimit.hit(&keyname)
            }
        };

        if within_limits {
            self.reply_ok().await;
        } else {
            self.reply_ko().await;
        }

        Ok(())
    }

    /// Handles a single command (one read currently)
    async fn handle_one(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut buffer = [0; 512];
        let read = self.stream.read(&mut buffer).await?;

        // Empty read: close the connection
        if read == 0 {
            return Err("".into());
        }

        let (command, keyname) = match read_input(buffer, read) {
            Ok(x) => x,
            Err(_) => {
                self.reply_err().await;
                return Ok(());
            }
        };

        match command {
            Command::INCR => {
                if self.handle_incr(&keyname).await.is_err() {
                    self.reply_err().await;
                }
            }
            // Unknown command
            // _ => {
            //     self.reply_err().await;
            // }
        }

        Ok(())
    }

    async fn main(&mut self) {
        loop {
            match self.handle_one().await {
                Ok(()) => (),
                Err(_) => {
                    break;
                },
            }
        }
    }
}


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
fn parse_specification(keyname: &str) -> Option<(u32, u32, String)> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^(\d+)/(\d+)_(.+)").unwrap();
    }

    let caps = RE.captures(&keyname)?;
    let hits = caps.get(1)?.as_str();
    let seconds = caps.get(2)?.as_str();
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
    assert_eq!(parse_specification(&"toto"), None);
    assert_eq!(parse_specification(&"1zzb_zo"), None);
    assert_eq!(parse_specification(&"1/2_toto"), Some((1, 2000, "toto".to_string())));
    assert_eq!(parse_specification(&"80/200_bar"), Some((80, 200_000, "bar".to_string())));
    assert_eq!(parse_specification(&"1/999999999999999_toto"), None);
    assert_eq!(parse_specification(&"99999999999999/99_toto"), None);
}

async fn cleanup_timer(rl_arc: Arc<Mutex<Ratelimit>>, meta_arc: Arc<Mutex<RatelimitCollection>>) {
    //println!("here?");
    let dur = Duration::from_millis(CLEANUP_INTERVAL);

    loop {
        task::sleep(dur).await;

        // let start = Instant::now();
        let _c=  {
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

        let arc =  Arc::new(Mutex::new(Ratelimit::new(HITS, DURATION_MS).unwrap()));
        let arc_collection =  Arc::new(Mutex::new(RatelimitCollection::new()));

        task::spawn(cleanup_timer(arc.clone(), arc_collection.clone()));

        let mut incoming = listener.incoming();
        while let Some(stream) = incoming.next().await {
            let mut handler = StreamHandler::new(stream?, &arc, &arc_collection);
            task::spawn( async move {
                handler.main().await
            });
        }
        Ok(())
    })
}
