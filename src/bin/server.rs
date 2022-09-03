use std::str;

use async_std::io;
use async_std::net::TcpListener;
use async_std::prelude::*;
use async_std::task;
use async_std::sync::Arc;

use futures::lock::Mutex;
use futures::stream::StreamExt;

use ratelimit_rs::Ratelimit;

const HITS: u32 = 5;
const DURATION_MS: u32 = 10_000;

fn main() -> io::Result<()> {
    task::block_on(async {
        let listener = TcpListener::bind("127.0.0.1:11211").await?;
        let ratelimit =  Arc::new(Mutex::new(Ratelimit::new(HITS, DURATION_MS)));

        let mut incoming = listener.incoming();
        while let Some(stream) = incoming.next().await {
            let mut stream = stream?;
            let ratelimit = ratelimit.clone();
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

                    let input = match str::from_utf8(&buffer[0..read]) {
                        Ok(v) => v,
                        Err(_) => break,
                    }.trim();

                    println!("incoming read: {:?}", input);
                    
                    // XXX handle "break" as error
                    let (command, key) = {
                        let mut split = input.split(" ");
                        (
                        match split.next() {
                                Some(x) => x.trim(),
                                None => break,
                            },
                        match split.next() {
                            Some(x) => x.trim(),
                            None => break,
                        })
                    };

                    let response = match command {
                        "incr" => {
                            let mut ratelimit = ratelimit.lock().await;
                            if ratelimit.hit(&key.to_string()) {
                                "0\r\n"
                            } else {
                                "1\r\n"
                            }
                        },
                        _ => "ERROR\r\n",
                    };
                    
                    stream.write(response.as_bytes() ).await;
                    stream.flush().await;
                    
                };
            });
            
        }
        Ok(())
    })
}
