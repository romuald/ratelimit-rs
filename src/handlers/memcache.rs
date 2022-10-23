use std::str;

use lazy_static::lazy_static;

use regex::Regex;

use async_std::prelude::*;
use async_std::sync::Arc;

use async_std::io::{Read, Write};
use futures::lock::Mutex;

use crate::{Ratelimit, RatelimitCollection};

pub trait AsyncStream: Read + Write + Unpin {}
impl<T: Read + Write + Unpin>  AsyncStream for T {}

enum Command {
    Incr(String),
}

pub struct StreamHandler {
    ratelimit: Arc<Mutex<Ratelimit>>,
    ratelimit_collection: Arc<Mutex<RatelimitCollection>>,
}


/// StreamHandler
/// Handles a single TCP stream
impl StreamHandler {
    pub fn new(
        ratelimit: &Arc<Mutex<Ratelimit>>,
        ratelimit_collection: &Arc<Mutex<RatelimitCollection>>,
    ) -> StreamHandler {
        StreamHandler {
            ratelimit: ratelimit.clone(),
            ratelimit_collection: ratelimit_collection.clone(),
        }
    }

    /// An "OK" response, request was within the limits (ironically 0)
    async fn reply_ok(&self, stream: &mut impl AsyncStream) -> bool {
        self.write("0\r\n", stream).await
    }

    /// An "not OK" response, request was outside the limits and should be limited (ironically 1)
    async fn reply_ko(&self, stream: &mut impl AsyncStream) -> bool {
        self.write("1\r\n", stream).await
    }

    /// An "error" response, the request was malformed or using a bad syntax
    async fn reply_err(&self, stream: &mut impl AsyncStream) -> bool {
        self.write("ERR\r\n", stream).await
    }

    /// Flush the binary response
    async fn write(&self, response: &str, stream: &mut impl AsyncStream) -> bool {
        stream.write(response.as_bytes()).await.is_ok() && stream.flush().await.is_ok()
    }

    /// Handles an "incr" command
    /// Will write the response on the output stream
    /// Can return an error in case the keyname is invalid
    async fn handle_incr(
        &self,
        keyname: &str,
        stream: &mut impl AsyncStream,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let within_limits = match parse_specification(keyname) {
            Some((hits, duration, keyname)) => {
                let mut meta = self.ratelimit_collection.lock().await;
                let rl = meta.get_instance(hits, duration)?;
                rl.hit(&keyname)
            }
            None => {
                let mut ratelimit = self.ratelimit.lock().await;
                ratelimit.hit(keyname)
            }
        };

        if within_limits {
            self.reply_ok(stream).await;
        } else {
            self.reply_ko(stream).await;
        }

        Ok(())
    }

    /// Handles a single command (one read currently)
    async fn handle_one(
        &self,
        stream: &mut impl AsyncStream,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut buffer = [0; 512];
        let read = stream.read(&mut buffer).await?;

        // Empty read: close the connection
        if read == 0 {
            return Err("".into());
        }

        let command = match read_input(buffer, read) {
            Ok(x) => x,
            Err(_) => {
                self.reply_err(stream).await;
                return Ok(());
            }
        };

        match command {
            Command::Incr(ref keyname) => {
                if self.handle_incr(keyname, stream).await.is_err() {
                    self.reply_err(stream).await;
                }
            } // Unknown command
              // _ => {
              //     self.reply_err().await;
              // }
        }

        Ok(())
    }

    pub async fn main(&self, stream: &mut impl AsyncStream) {
        #[cfg(test)]
        let mut tmax = 1_000;

        loop {
            #[cfg(test)]
            {
                // Avoid an infinite loop in tests
                tmax -= 1;
                if tmax == 0 {
                    break;
                }
            }

            if self.handle_one(stream).await.is_err() {
                break;
            }
        }
    }
}

fn read_input(buffer: [u8; 512], len: usize) -> Result<Command, ()> {
    let input = match str::from_utf8(&buffer[0..len]) {
        Ok(v) => v,
        Err(_) => return Err(()),
    }
    .trim();

    let (command, key) = {
        let mut split = input.split(' ');
        (
            match split.next() {
                Some(x) => x.trim(),
                None => return Err(()),
            },
            match split.next() {
                Some(x) => x.trim(),
                None => return Err(()),
            },
        )
    };

    match command {
        "incr" => Ok(Command::Incr(String::from(key))),
        _ => Err(()),
    }
}

/// Parse a specification returning: `(hits, duration, keyname)`
///
/// ## Example
///
/// ```ignored
/// let keyname = "1/2_foo";
/// let result = parse_specification(keyname);
/// assert_eq!(Some((1, 2_000, "foo")), result);
/// ```
fn parse_specification(keyname: &str) -> Option<(u32, u32, String)> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^(\d+)/(\d+)_(.+)").unwrap();
    }

    let caps = RE.captures(keyname)?;
    let hits = caps.get(1)?.as_str();
    let seconds = caps.get(2)?.as_str();
    let keyname = caps.get(3)?.as_str().to_string();

    let hits = hits.parse();
    let seconds = seconds.parse::<u32>();

    if let (Ok(hits), Ok(seconds)) = (hits, seconds) {
        Some((hits, seconds * 1000, keyname))
    } else {
        None
    }
}

/*

enum ProtocolResponse {
    Valid(String),
    Empty,
    Error,
    Fatal,
}

async fn handle_one(stream: &mut TcpStream)  -> Result<ProtocolResponse, Box<dyn std::error::Error>>{
    let mut buffer = [0; 512];
    let read = stream.read(&mut buffer).await?;

    // Empty read: close the connection
    if read == 0 {
        return Err("".into());
    }
    let command = read_input(buffer, read);
    if command.is_err() {
        return Ok(ProtocolResponse::Error);
    }

    let command = command.unwrap();
    match command {
        Command::INCR(x) => todo!(),
    }


    Ok(ProtocolResponse::Valid("OK".into()))
}

async fn handle_stream(stream: &mut TcpStream) {
    loop {
        let result = handle_one(stream).await;
        if result.is_err() {
            break;
        }
        let result = result.unwrap();
        let response =  match result {
            ProtocolResponse::Valid(ref x) =>x.as_bytes(),
            ProtocolResponse::Error => "ERR\r\n".as_bytes(),
            ProtocolResponse::Empty => continue,
            ProtocolResponse::Fatal => break,
        };

        if stream.write(response).await.is_err() {
            break;
        }
    }

}
*/

#[cfg(test)]
mod test {
    use super::*;

    use crate::testing::MockTcpStream;
    use mock_instant::MockClock;

    #[test]
    fn test_parse_specification() {
        assert_eq!(parse_specification("toto"), None);
        assert_eq!(parse_specification("1zzb_zo"), None);
        assert_eq!(
            parse_specification("1/2_toto"),
            Some((1, 2000, "toto".to_string()))
        );
        assert_eq!(
            parse_specification("80/200_bar"),
            Some((80, 200_000, "bar".to_string()))
        );
        assert_eq!(parse_specification("1/999999999999999_toto"), None);
        assert_eq!(parse_specification("99999999999999/99_toto"), None);
    }

    #[async_std::test]
    async fn test_base() {
        let root = std::time::Duration::from_millis(86_400_000);
        MockClock::set_time(root);

        let rl = Arc::new(Mutex::new(Ratelimit::new(1, 1).unwrap()));
        let xrl = Arc::new(Mutex::new(RatelimitCollection::default()));
        let handler = StreamHandler::new(&rl, &xrl);

        let mut stream = MockTcpStream::from_rdata("incr zzz\r\n".to_string());

        handler.handle_one(&mut stream).await.unwrap();

        assert_eq!(stream.get_wdata(), "0\r\n");
    }
}
