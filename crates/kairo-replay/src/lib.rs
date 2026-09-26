//! Stores bounded snapshots independently of Lua, physics, files, and GPU resources.
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rng {
    state: u64,
}

impl Default for Rng {
    fn default() -> Self {
        Self::seeded(0)
    }
}
impl Rng {
    pub fn seeded(seed: u64) -> Self {
        Self { state: seed }
    }
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    pub fn float(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1_u64 << 53) as f64
    }
    pub fn integer(&mut self, min: i64, max: i64) -> Result<i64> {
        ensure!(min <= max, "random integer minimum exceeds maximum");
        let span = (i128::from(max) - i128::from(min) + 1) as u128;
        if span == (1_u128 << 64) {
            return Ok(self.next_u64() as i64);
        }
        let span = span as u64;
        let threshold = span.wrapping_neg() % span;
        loop {
            let sample = self.next_u64();
            if sample >= threshold {
                return Ok((i128::from(min) + i128::from(sample % span)) as i64);
            }
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SnapshotInfo {
    pub frame: u64,
    pub time: f64,
    pub bytes: usize,
}

struct Snapshot {
    info: SnapshotInfo,
    bytes: Vec<u8>,
}

pub struct Timeline {
    snapshots: VecDeque<Snapshot>,
    seconds: f64,
    budget: usize,
    bytes: usize,
}

impl Timeline {
    pub fn new(seconds: f64, budget: usize) -> Result<Self> {
        ensure!(
            seconds.is_finite() && (0.1..=120.0).contains(&seconds),
            "replay duration must be 0.1..=120 seconds"
        );
        ensure!(
            (1024..=256 * 1024 * 1024).contains(&budget),
            "replay budget must be 1 KiB..=256 MiB"
        );
        Ok(Self {
            snapshots: VecDeque::new(),
            seconds,
            budget,
            bytes: 0,
        })
    }
    pub fn capture(&mut self, frame: u64, time: f64, bytes: Vec<u8>) -> Result<()> {
        ensure!(time.is_finite() && time >= 0.0, "invalid snapshot time");
        ensure!(
            bytes.len() <= self.budget,
            "one replay snapshot exceeds the entire buffer budget"
        );
        if let Some(last) = self.snapshots.back() {
            ensure!(
                frame >= last.info.frame && time >= last.info.time,
                "snapshot time moved backwards; branch the timeline first"
            );
        }
        if self
            .snapshots
            .back()
            .is_some_and(|last| last.info.frame == frame)
        {
            if let Some(last) = self.snapshots.pop_back() {
                self.bytes -= last.bytes.len();
            }
        }
        while self.snapshots.front().is_some_and(|first| {
            time - first.info.time > self.seconds
                || self.bytes + bytes.len() > self.budget
                || self.snapshots.len() >= 7200
        }) {
            if let Some(first) = self.snapshots.pop_front() {
                self.bytes -= first.bytes.len();
            }
        }
        self.bytes += bytes.len();
        self.snapshots.push_back(Snapshot {
            info: SnapshotInfo {
                frame,
                time,
                bytes: bytes.len(),
            },
            bytes,
        });
        Ok(())
    }
    /// Prepare a new branch before modifying any live simulation state.
    pub fn replacement(&self, frame: u64, time: f64, bytes: Vec<u8>) -> Result<Self> {
        let mut replacement = Self::new(self.seconds, self.budget)?;
        replacement.capture(frame, time, bytes)?;
        Ok(replacement)
    }
    pub fn at_or_before(&self, time: f64) -> Result<(SnapshotInfo, Vec<u8>)> {
        ensure!(time.is_finite(), "invalid replay target time");
        let snapshot = self
            .snapshots
            .iter()
            .rev()
            .find(|snapshot| snapshot.info.time <= time)
            .or_else(|| self.snapshots.front())
            .context("no replay snapshots are available")?;
        Ok((snapshot.info.clone(), snapshot.bytes.clone()))
    }
    pub fn at_index(&self, index: usize) -> Result<(SnapshotInfo, Vec<u8>)> {
        let snapshot = self
            .snapshots
            .get(index)
            .context("replay snapshot index out of range")?;
        Ok((snapshot.info.clone(), snapshot.bytes.clone()))
    }
    pub fn branch(&mut self, frame: u64) {
        while self
            .snapshots
            .back()
            .is_some_and(|last| last.info.frame > frame)
        {
            if let Some(last) = self.snapshots.pop_back() {
                self.bytes -= last.bytes.len();
            }
        }
    }
    pub fn clear(&mut self) {
        self.snapshots.clear();
        self.bytes = 0;
    }
    pub fn infos(&self) -> Vec<SnapshotInfo> {
        self.snapshots
            .iter()
            .map(|snapshot| snapshot.info.clone())
            .collect()
    }
    pub fn bytes(&self) -> usize {
        self.bytes
    }
    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rng_has_a_known_sequence_and_can_restore_exactly() {
        let mut rng = Rng::default();
        assert_eq!(rng.next_u64(), 0xe220_a839_7b1d_cdaf);
        let saved = rng.clone();
        let value = rng.next_u64();
        rng = saved;
        assert_eq!(rng.next_u64(), value);
        for _ in 0..1000 {
            assert!((-3..=7).contains(&rng.integer(-3, 7).unwrap()));
        }
    }
    #[test]
    fn byte_budget_and_time_window_evict_old_snapshots() {
        let mut timeline = Timeline::new(1.0, 1024).unwrap();
        timeline.capture(1, 0.0, vec![1; 700]).unwrap();
        timeline.capture(2, 0.2, vec![2; 700]).unwrap();
        assert_eq!(timeline.infos().len(), 1);
        assert_eq!(timeline.at_or_before(0.0).unwrap().0.frame, 2);
        assert!(timeline.capture(3, 0.3, vec![0; 1025]).is_err());
        assert_eq!(timeline.bytes(), 700);
    }
    #[test]
    fn branching_discards_the_abandoned_future() {
        let mut timeline = Timeline::new(10.0, 4096).unwrap();
        for frame in 0..10 {
            timeline.capture(frame, frame as f64, vec![0; 10]).unwrap();
        }
        timeline.branch(4);
        assert_eq!(timeline.infos().len(), 5);
        timeline.capture(5, 4.5, vec![1; 10]).unwrap();
        assert_eq!(timeline.at_index(5).unwrap().1, vec![1; 10]);
    }
    #[test]
    fn oversized_bookmark_replacement_does_not_erase_live_history() {
        let mut timeline = Timeline::new(10.0, 1024).unwrap();
        timeline.capture(8, 1.0, vec![7; 128]).unwrap();
        assert!(timeline.replacement(2, 0.2, vec![0; 1025]).is_err());
        assert_eq!(timeline.at_index(0).unwrap().0.frame, 8);
        let replaced = timeline.replacement(2, 0.2, vec![1; 64]).unwrap();
        assert_eq!(replaced.at_index(0).unwrap().0.frame, 2);
        assert_eq!(timeline.bytes(), 128);
    }
}
