use std::collections::HashMap;

use crate::ratelimit::{Ratelimit,RatelimitInvalidError};

pub struct RatelimitCollection {
    entries: HashMap<(u32, u32), Ratelimit>,
}

impl RatelimitCollection {
    pub fn new() -> RatelimitCollection {
        RatelimitCollection { entries: HashMap::new() }
    }

    pub fn get_instance(&mut self, hits: u32, duration: u32) -> Result<&mut Ratelimit, RatelimitInvalidError> {
        if ! self.entries.contains_key(&(hits, duration)) {
            let rl = Ratelimit::new(hits, duration)?;
            self.entries.insert((hits, duration), rl);
        }
        Ok(self.entries.get_mut(&(hits, duration)).unwrap())
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
    use std::time::Duration;
    use mock_instant::{MockClock};

    #[test]
    fn test_collection_cleanup() {
        let root = Duration::from_millis(86_400_000);

        MockClock::set_time(root);

        let mut meta = RatelimitCollection::new();
        meta.get_instance(1, 1000).unwrap().hit("foo");
        meta.get_instance(10, 1_000).unwrap().hit("bar");
        meta.get_instance(8, 10_000).unwrap().hit("bar");

        MockClock::advance(Duration::from_secs(6));

        assert_eq!(meta.cleanup(), 2);
    }
}
