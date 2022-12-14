#[cfg(not(test))]
use std::time::Instant;

#[cfg(test)]
use mock_instant::Instant;

use rayon::prelude::*;
use std::collections::HashMap;

use crate::ratelimit::{Ratelimit, RatelimitInvalidError};

#[derive(Default)]
pub struct RatelimitCollection {
    entries: HashMap<(u32, u32), Ratelimit>,
}

impl RatelimitCollection {
    pub fn get_instance(
        &mut self,
        hits: u32,
        duration: u32,
    ) -> Result<&mut Ratelimit, RatelimitInvalidError> {
        #[allow(clippy::map_entry)]
        if !self.entries.contains_key(&(hits, duration)) {
            let rl = Ratelimit::new(hits, duration)?;
            self.entries.insert((hits, duration), rl);
        }

        Ok(self.entries.get_mut(&(hits, duration)).unwrap())
    }

    pub fn cleanup(&mut self) -> usize {
        let now = Instant::now();
        self.entries
            .par_iter_mut()
            .map(|(_, val)| val.cleanup_at(now))
            .sum()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use mock_instant::MockClock;
    use std::time::Duration;

    #[test]
    fn test_collection_cleanup() {
        let root = Duration::from_millis(86_400_000);

        MockClock::set_time(root);

        let mut meta = RatelimitCollection::default();
        meta.get_instance(1, 1000).unwrap().hit("foo");
        meta.get_instance(10, 1_000).unwrap().hit("bar");
        meta.get_instance(8, 10_000).unwrap().hit("bar");

        MockClock::advance(Duration::from_secs(6));

        assert_eq!(meta.cleanup(), 2);
    }
}
