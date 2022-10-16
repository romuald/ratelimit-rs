#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]
use ratelimit_rs::Ratelimit;
use std::io;
use std::time::{Duration, Instant};

use std::thread::sleep;

struct InputReader {
    input: std::io::Stdin,
}

fn inputreader() -> InputReader {
    InputReader { input: io::stdin() }
}

impl Iterator for InputReader {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buffer = String::from("");

        match self.input.read_line(&mut buffer) {
            Ok(_) => {
                let value = buffer.trim();

                match value.len() {
                    0 => None,
                    _ => Some(value.to_string()),
                }
            }
            Err(_) => None,
        }
    }
}

fn main() -> io::Result<()> {
    let mut rl = Ratelimit::new(5, 10000).unwrap();

    for keyname in inputreader() {
        if rl.hit(&keyname) {
            println!("OK for {:?}", keyname);
        } else {
            println!("Not OK for {:?}", keyname);
        }
    }

    /*
    loop {
        buffer.clear();

        match stdin.read_line(&mut buffer) {
            Ok(n) => {
                let value = buffer.trim();
                if value.len() == 0 { break }
            }
            Err(_e) => break,
        }
    }
    */

    Ok(())
}
fn demo() {
    let mut rl = Ratelimit::new(50, 100).unwrap();
    let mut b: bool;

    let name = &String::from("hello");

    for _i in 0..3 {
        b = rl.hit(&name);
        println!("@{:?}, {:?}", line!(), b);
    }
    sleep(Duration::from_millis(150));

    b = rl.hit(&name);
    println!("@{:?}, {:?}", line!(), b);

    b = rl.hit(&name);
    println!("@{:?}, {:?}", line!(), b);
}
