use std::time::{Duration, Instant};

use std::collections::HashMap;
use std::convert::TryFrom;

use std::cmp;

const BLOCK_SIZE: usize = 32;

struct Entry {
    epoch: Instant,
    index: u32,
    timestamps: Vec<u32>,
}

impl Entry {
    fn new() -> Entry {
        Entry {
            epoch: Instant::now() - Duration::from_millis(1),
            index: 0,
            timestamps: vec![0; BLOCK_SIZE],
        }
    }
    /// Hits the ratelimit
    /// size is the maximum number of hits allowed
    /// duration is the duration in milliseconds for which the hits are allowed
    fn hit(&mut self, size: u32, duration: u32) -> bool {
        let diff = Instant::now().duration_since(self.epoch);
        let now = u32::try_from(diff.as_millis()).unwrap();
    
        let index = usize::try_from(self.index).unwrap();
        if index == self.timestamps.len() {
            let max = usize::try_from(size).unwrap();
            let increment = cmp::min(BLOCK_SIZE, max - index);
            self.timestamps.extend(vec![0; increment]);
        }

        let last = self.timestamps[index];
        //println!("ts: {:?}, now: {:?}, diff: {:?}", self.timestamps, now, diff);
        let delta = now - last;

        // println!("delta: {:?} index {:?}, last {:?}, period: {:?}", delta, self.index, last, period);
        if last > 0 && delta < duration {
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
    duration: u32,
    entries: HashMap<String, Entry>,
}

impl Ratelimit {
    pub fn new(hits: u32, duration: u32) -> Ratelimit {
        Ratelimit {
            hits: hits,
            duration: duration,
            entries: HashMap::new(),
        }
    }

    pub fn hit(&mut self, name: &String) -> bool {
        return match self.entries.get_mut(name) {
            Some(entry) => {
                entry.hit(self.hits, self.duration)
            }
            None => {
                let mut new_entry = Entry::new();
                new_entry.hit(self.hits, self.duration);
                self.entries.insert(name.clone(), new_entry);
                true // assumes that we are not limited to 0 hits
            }
        }
    }
}
