use std::cmp;
use std::time::Duration;

#[cfg(not(test))]
use std::time::Instant;

#[cfg(test)]
use mock_instant::Instant;

use std::collections::HashMap;
use std::convert::TryFrom;

const BLOCK_SIZE: usize = 64;
const MAX_DURATION: u32 = 86400 * 48 * 1000; // 48 days, ~49 days being the number of milliseconds that fits in a u32

struct RLEntry {
    epoch: Instant,
    index: u32,
    timestamps: Vec<u32>,
}

impl RLEntry {
    fn new() -> RLEntry {
        RLEntry {
            epoch: Instant::now() - Duration::from_millis(1),
            index: 0,
            timestamps: vec![0; BLOCK_SIZE],
        }
    }

    // now is the difference since epoch
    fn rebase(&mut self, now: u32) -> u32{
        // let epoch = self.epoch;

        if now < u32::MAX / 2 {
            return now;
        }
        /*
        println!(
            "Rebase epoch={:?}, now={:?}",
            epoch, now
        );
        println!("Values: {:?}", self.timestamps);
         */

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

        /* println!("new values: {:?}", self.timestamps);
        println!("min: {:?}, new={:?}", min, new_epoch); */

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
    entries: HashMap<String, RLEntry>,
}

#[derive(Debug, Clone)]
pub struct RatelimitInvalidError {
    hits: u32,
    duration: u32,
}

impl std::fmt::Display for RatelimitInvalidError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.hits == 0 {
            write!(f, "Invalid ratelimit specification, hits must be greater than 0")    
        } else if self.duration == 0 {
            write!(f,  "Invalid ratelimit specification, duration must be greater than 0")
        } else {
            write!(f,  "Invalid ratelimit specification, duration must be less than {:?}", MAX_DURATION)
        }
    }
}

impl Ratelimit {
    pub fn check_bounds(hits: u32, duration: u32) -> Result<(), RatelimitInvalidError> {
        if hits == 0 || duration == 0 || duration > MAX_DURATION {
            Err(RatelimitInvalidError{hits: hits, duration: duration})
        } else {
            Ok(())
        }
    }

    pub fn new(hits: u32, duration: u32) -> Result<Ratelimit, RatelimitInvalidError> {
        Ratelimit::check_bounds(hits, duration)?;
        
        Ok(Ratelimit {
            hits: hits,
            duration: duration,
            entries: HashMap::new(),
        })
    }

    pub fn hit(&mut self, name: &str) -> bool {
        match self.entries.get_mut(name) {
            Some(entry) => entry.hit(self.hits, self.duration),
            None => {
                let mut new_entry = RLEntry::new();
                new_entry.hit(self.hits, self.duration);
                self.entries.insert(name.to_string(), new_entry);
                true // assumes that we are not limited to 0 hits
            }
        }
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn cleanup(&mut self) -> usize {
        let min = Instant::now() - Duration::from_millis(1000 + u64::from(self.duration));
        let before = self.entries.len();

        self.entries.retain(|_, v| {
            // println!("{:?}", v.timestamps);
            let index = usize::try_from(v.index).unwrap();

            let last = match index {
                0 => v.timestamps.len() - 1,
                _ => index - 1,
            };

            let last_ts = v.epoch + Duration::from_millis(u64::from(v.timestamps[last]));

            last_ts > min
        });
        //self.entries.shrink_to_fit();
        before - self.entries.len()
    }
}

pub struct MetaRatelimit {
    entries: HashMap<(u32, u32), Ratelimit>,
}

impl MetaRatelimit {
    pub fn new() -> MetaRatelimit {
        MetaRatelimit { entries: HashMap::new() }
    }

    pub fn get_instance(&mut self, hits: u32, duration: u32) -> Result<&mut Ratelimit, RatelimitInvalidError> {
        if ! self.entries.contains_key(&(hits, duration)) {
            let rl = Ratelimit::new(hits, duration)?;
            self.entries.insert((hits, duration), rl);
        }
        Ok(self.entries.get_mut(&(hits, duration)).unwrap())
    }

    pub fn hit(&mut self, hits: u32, duration: u32, key: &str) -> bool {
        // Warning: panics if called out of bounds
        match self.entries.get_mut(&(hits, duration)) {
            Some(e) =>  e.hit(&key),
            None => {
                let mut rl = Ratelimit::new(hits, duration).unwrap();
                let ret = rl.hit(&key);
                self.entries.insert((hits, duration), rl);
                ret
            }
        }
    }

    pub fn cleanup(&mut self) -> usize {
        let mut sum = 0;

        for (_, val) in self.entries.iter_mut() {
            sum += val.cleanup();
        }

        sum
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

        let mut rl = Ratelimit::new(10, rl_duration_ms).unwrap();
        let st = "test";

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

        let mut rl = Ratelimit::new(1, 86400 * 1000).unwrap();
        let st = "test";

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

        let mut rl = Ratelimit::new(10, rl_duration_ms).unwrap();

        rl.hit("foo");
        rl.hit("bar");

        MockClock::advance(Duration::from_millis(59_000));
        rl.hit("bar");

        rl.cleanup();
        assert_eq!(rl.entries.len(), 2);

        MockClock::advance(Duration::from_millis(3_000));

        rl.cleanup();
        assert_eq!(rl.entries.len(), 1);

        MockClock::advance(Duration::from_millis(59_000));

        rl.cleanup();
        assert_eq!(rl.entries.len(), 0);

    }

    #[test]
    fn test_meta_cleanup() {
        let root = Duration::from_millis(86_400_000);

        MockClock::set_time(root);

        let mut meta = MetaRatelimit::new();
        meta.get_instance(1, 1000).unwrap().hit(("foo"));
        meta.get_instance(10, 1_000).unwrap().hit("bar");
        meta.get_instance(8, 10_000).unwrap().hit("bar");

        MockClock::advance(Duration::from_secs(6));

        assert_eq!(meta.cleanup(), 2);
    }

    #[test]
    fn test_bounds() {
        let fail = Ratelimit::new(0, 10);
        assert!(fail.is_err());
        let fail = Ratelimit::new(10, 0);
        assert!(fail.is_err());

        let fail = Ratelimit::new(10, 2 << 32 - 1);
        assert!(fail.is_err());
    }
}
