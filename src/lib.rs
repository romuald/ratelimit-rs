use std::time::Duration;

#[cfg(not(test))]
use std::time::Instant;

#[cfg(test)]
use mock_instant::Instant;
//use fake_instant::FakeClock as Instant;

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

    // now is the difference since epoch
    fn rebase(&mut self, now: u32) -> u32{
        let epoch = self.epoch;

        if now < u32::MAX / 2 {
            return now;
        }

        println!(
            "Rebase epoch={:?}, now={:?}",
            epoch, now
        );
        println!("Values: {:?}", self.timestamps);

        let min :u32 = match self.timestamps.iter().filter(|x| **x > 0).min() {
            Some(x) => *x - 1,
            None => 0,
        };


        let new_epoch = self.epoch + Duration::from_millis(min.into());
        for timestamp in self.timestamps.iter_mut() {
            if *timestamp > 0 {
                *timestamp -= min;
            }
        }
        self.epoch = new_epoch;
        println!("new values: {:?}", self.timestamps);


        println!("min: {:?}, new={:?}", min, new_epoch);

        return now - min;
    }

    /// Hits the ratelimit
    /// size is the maximum number of hits allowed
    /// duration is the duration in milliseconds for which the hits are allowed
    fn hit(&mut self, size: u32, duration: u32) -> bool {
        let diff = Instant::now().duration_since(self.epoch);
        let mut now = u32::try_from(diff.as_millis()).unwrap();

        let index = usize::try_from(self.index).unwrap();

        {
            if index == self.timestamps.len() {
                let max = usize::try_from(size).unwrap();
                let increment = cmp::min(BLOCK_SIZE, max - index);
                self.timestamps.extend(vec![0; increment]);
            }
        }

        now = self.rebase(now);

        let previous = self.timestamps[index];
        //println!("ts: {:?}, now: {:?}, diff: {:?}", self.timestamps, now, diff);

        // println!("delta: {:?} index {:?}, previous {:?}, period: {:?}", delta, self.index, previous, period);
        if previous > 0 && (now - previous) < duration {
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
        // Assert hits > 0
        // Assert duration < 49 days
        Ratelimit {
            hits: hits,
            duration: duration,
            entries: HashMap::new(),
        }
    }

    pub fn hit(&mut self, name: &String) -> bool {
        match self.entries.get_mut(name) {
            Some(entry) => entry.hit(self.hits, self.duration),
            None => {
                let mut new_entry = Entry::new();
                new_entry.hit(self.hits, self.duration);
                self.entries.insert(name.clone(), new_entry);
                true // assumes that we are not limited to 0 hits
            }
        }
    }

    pub fn cleanup(&mut self) {
        let min = Instant::now() - Duration::from_millis(1000 + u64::from(self.duration));

        self.entries.retain(|_, v| {
            println!("{:?}", v.timestamps);
            let index = usize::try_from(v.index).unwrap();

            let last = match index {
                0 => v.timestamps.len() - 1,
                _ => index - 1,
            };

            let last_ts = v.epoch + Duration::from_millis(u64::from(v.timestamps[last]));

            last_ts > min
        });
    }
}

#[cfg(test)]

mod test {
    use super::*;
    use mock_instant::{MockClock};

    #[test]
    fn test_base_process() {
        // Basic test "suite", hitting the ratelimit within a
        // specific timeframe will either return true or false
        let ms1 = Duration::from_millis(1);
        let root = ms1.clone();
        let rl_duration_ms = 2_000;
        let rl_duration = Duration::from_millis(rl_duration_ms.into());

        MockClock::set_time(root);

        let mut rl = Ratelimit::new(10, rl_duration_ms);
        let st = String::from("test");

        // 10 hits OK in 1 second
        for _ in 0..10 {
            assert_eq!(rl.hit(&st), true);

            MockClock::advance(Duration::from_millis(10));
        }

        // still not OK
        assert_eq!(rl.hit(&st), false);

        // still not OK less than 2 seconds from the start
        MockClock::set_time(root + rl_duration - ms1);
        assert_eq!(rl.hit(&st), false);

        // 
        MockClock::advance(Duration::from_millis(1));
        assert_eq!(rl.hit(&st), true);
    }

    #[test]
    fn test_overflow() {
        // 
        MockClock::set_time(Duration::from_millis(200));

        let mut rl = Ratelimit::new(1, 86400 * 1000);
        let st = String::from("test");

        MockClock::set_time(Duration::from_millis(200));

        for _i in 0..70 {
            assert!(rl.hit(&st));
            MockClock::advance(Duration::from_secs(86400));
        }
    }

    #[test]
    fn test_cleanup() {
        let root = Duration::from_millis(86_400_000);
        let rl_duration_ms = 60_000;

        MockClock::set_time(root);

        let mut rl = Ratelimit::new(10, rl_duration_ms);

        rl.hit(&String::from("foo"));
        rl.hit(&String::from("bar"));

        MockClock::advance(Duration::from_millis(59_000));
        rl.hit(&String::from("bar"));

        rl.cleanup();
        assert_eq!(rl.entries.len(), 2);

        MockClock::advance(Duration::from_millis(3_000));

        rl.cleanup();
        assert_eq!(rl.entries.len(), 1);

        MockClock::advance(Duration::from_millis(59_000));

        rl.cleanup();
        assert_eq!(rl.entries.len(), 0);

    }
}
