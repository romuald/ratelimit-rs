use futures::stream::StreamExt;
 
use async_std::io;
use async_std::net::TcpListener;
use async_std::prelude::*;
use async_std::task;
 
use futures::lock::Mutex;
use async_std::sync::Arc;
use std::str;

use ratelimit_rs::Ratelimit;


fn main() -> io::Result<()> {
    task::block_on(async {
        let listener = TcpListener::bind("127.0.0.1:7878").await?;
        let rl =  Arc::new(Mutex::new((Ratelimit::new(5, 10_000))));
          
        let mut incoming = listener.incoming();
        while let Some(stream) = incoming.next().await {
            let mut stream = stream?;
            let rl = rl.clone();
            task::spawn(async move {
                const M: usize = 1024;
                let mut buffer = [0; M];
                let mut err = false;
                loop {
                    let read = match stream.read(&mut buffer).await {
                        Ok(n) => n,
                        Err(_) => break,
                    };

                    if read == 0 {
                        break;
                    }

                    let input = match str::from_utf8(&buffer[0..read]) {
                        Ok(v) => v,
                        Err(_) => break,
                    }.trim();

                    //let input = input.trim_matches(char::is_whitespace).trim().to_string();
/*
                    let eos = match input.find('\0') {
                        Some(x) => x - 1,
                        None => input.len(),
                    };

                    let input = String::from(&input[0..eos]);
*/
                    //let input = String::from(String::from_utf8(buffer.into()).unwrap().trim());

                    println!("did read {:?}", input);
                    /*
                    let (a, b) = match input.split_once(" ") {
                        Some((x, y)) => (x.trim(), y.trim()),
                        None => break,
                    };
                    */
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
                   
                    if command == "INC" {
                        println!("inc ok: {:?}", key);
                    }
                    
                    let response = { // block for scoping lock.
                        let mut rl = rl.lock().await;
                        let valid = rl.hit(&key.to_string());
                        format!("input: {:?}, valid: {:?}\n", input, valid)
                    };
                    stream.write(response.as_bytes()).await;
                    stream.flush().await;
                };

                if ( err ) {
                    stream.write("ERR\r\n".as_bytes() ).await;
                    stream.flush().await;
                }
            });
            
        }
        Ok(())
    })
}
