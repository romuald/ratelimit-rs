#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]
use ratelimit_rs::Ratelimit;
use std::time::{Duration, Instant};

use std::thread::sleep;


fn main() {
    let mut rl = Ratelimit::new(19, 100);

    let mut b: bool;

    let name = &String::from("hello");

    for i in 0..20 {
        b = rl.hit(&name);
        println!("@{:?}, {:?}", line!(), b);
    }


}
