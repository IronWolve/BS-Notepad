use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

/// Acknowledging a conflict never changes the baseline used by Save.
#[derive(Clone, Default)]
pub struct State {
    pub epoch: u64,
    observed: Option<Option<u64>>,
    acknowledged: Option<Option<u64>>,
}
impl State {
    pub fn observe(&mut self, baseline: u64, observed: Option<u64>) -> bool {
        self.observed = Some(observed);
        if observed == Some(baseline) {
            self.acknowledged = None;
            false
        } else {
            self.acknowledged != Some(observed)
        }
    }
    pub fn acknowledge(&mut self) {
        self.acknowledged = self.observed;
    }
    pub fn saved(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        self.observed = None;
        self.acknowledged = None;
    }
}

#[derive(PartialEq)]
struct Stamp {
    length: u64,
    modified: SystemTime,
}
impl Stamp {
    fn read(path: &Path) -> Option<Self> {
        let metadata = std::fs::metadata(path).ok()?;
        if !metadata.is_file() {
            return None;
        }
        Some(Self {
            length: metadata.len(),
            modified: metadata.modified().ok()?,
        })
    }
}

/// Cache just the active path. Periodic hashing catches writers that retain timestamps.
#[derive(Default)]
pub struct Monitor {
    cached: Option<(PathBuf, Stamp, Option<u64>, Instant)>,
}
impl Monitor {
    pub fn check(&mut self, path: &Path, force: bool) -> Option<u64> {
        let stamp = Stamp::read(path);
        if !force {
            if let (Some(stamp), Some((prior_path, prior_stamp, hash, time))) =
                (&stamp, &self.cached)
            {
                if path == prior_path
                    && stamp == prior_stamp
                    && time.elapsed() < Duration::from_secs(30)
                {
                    return *hash;
                }
            }
        }
        let hash = crate::storage::disk_fingerprint(path).ok();
        // If it changed while being read, do not cache an inconsistent observation.
        self.cached = stamp
            .filter(|before| Some(before) == Stamp::read(path).as_ref())
            .map(|stamp| (path.to_owned(), stamp, hash, Instant::now()));
        hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn acknowledgement_survives_polls_but_not_another_disk_change() {
        let mut state = State::default();
        assert!(state.observe(1, Some(2)));
        state.acknowledge();
        assert!(!state.observe(1, Some(2)));
        assert!(state.observe(1, Some(3)));
        assert!(state.observe(1, None));
        state.acknowledge();
        assert!(!state.observe(1, None));
        assert!(!state.observe(1, Some(1)));
        assert!(state.observe(1, None));
        state.saved();
        assert_eq!(state.epoch, 1);
    }
}
