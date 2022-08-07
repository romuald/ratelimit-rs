use std::time::{Duration, Instant};

use std::collections::HashMap;
use std::convert::TryFrom;

use std::cmp;

struct Entry {
    epoch: Instant,
    index: u32,
    timestamps: Vec<u32>,
}

const SIZE_BLOCK: usize = 6;

impl Entry {
    fn new() -> Entry {
        Entry {
            epoch: Instant::now() - Duration::from_millis(1),
            index: 0,
            timestamps: vec![0; SIZE_BLOCK],
        }
    }

    fn hit(&mut self, size: u32, period: u32) -> bool {
        //let x : usize  = usize::from(self.index);
        //let mut ts = self.timestamps.clone();
        //ts[x] = 2;
        //let now = Instant::now();
        let diff = Instant::now().duration_since(self.epoch);
        let now = u32::try_from(diff.as_millis()).unwrap();
    
        let index = usize::try_from(self.index).unwrap();
        if index == self.timestamps.len() {
            let max = usize::try_from(size).unwrap();
            let increment = cmp::min(SIZE_BLOCK, max - index);
            self.timestamps.extend(vec![0; increment]);
        }

        let last = self.timestamps[index];
        //println!("ts: {:?}, now: {:?}, diff: {:?}", self.timestamps, now, diff);
        let delta = now - last;

        // println!("delta: {:?} index {:?}, last {:?}, period: {:?}", delta, self.index, last, period);
        if last > 0 && delta < period {
            println!("false?");
            return false;
        } else {
            self.timestamps[index] = now;
            if self.index == size - 1 {
                self.index = 0;
            } else {
                self.index += 1;
            }
        }
        //println!("ts: {:?}", self.timestamps);

        return true;
    }
}

pub struct Ratelimit {
    hits: u32,
    period: u32,
    entries: HashMap<String, Entry>,
}

impl Ratelimit {
    pub fn new(hits: u32, period: u32) -> Ratelimit {
        Ratelimit {
            hits: hits,
            period: period,
            entries: HashMap::new(),
        }
    }

    pub fn hit(&mut self, name: &String) -> bool {
        return match self.entries.get_mut(name) {
            Some(entry) => {
                entry.hit(self.hits, self.period)
            }
            None => {
                let mut new_entry = Entry::new();
                let ret = new_entry.hit(self.hits, self.period);
                self.entries.insert(name.clone(), new_entry);
                ret
            }
        }
    }
}
