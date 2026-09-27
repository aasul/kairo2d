use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BookmarkInfo {
    pub id: u64,
    pub label: String,
    pub note: String,
    pub scene: String,
    pub frame: u64,
    pub time: f64,
    pub bytes: usize,
    pub profile: crate::profiler::ProfileSample,
}
impl BookmarkInfo {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.id > 0
                && !self.label.is_empty()
                && self.label.len() <= 96
                && self.note.len() <= 512
                && self.scene.len() <= 64,
            "invalid bookmark metadata"
        );
        ensure!(
            self.time.is_finite() && self.time >= 0.0,
            "invalid bookmark time"
        );
        Ok(())
    }
}
struct Entry {
    info: BookmarkInfo,
    snapshot: Vec<u8>,
}
#[derive(Default)]
pub struct Bookmarks {
    entries: VecDeque<Entry>,
    next: u64,
    bytes: usize,
}
impl Bookmarks {
    pub fn record(&mut self, mut info: BookmarkInfo, snapshot: Vec<u8>) -> Result<u64> {
        let id = self.next.checked_add(1).context("bookmark IDs exhausted")?;
        info.id = id;
        info.bytes = snapshot.len();
        info.validate()?;
        ensure!(
            snapshot.len() <= 16 * 1024 * 1024,
            "bookmark snapshot exceeds 16 MiB"
        );
        while self.entries.len() >= 8 || self.bytes + snapshot.len() > 16 * 1024 * 1024 {
            if let Some(old) = self.entries.pop_front() {
                self.bytes -= old.snapshot.len();
            } else {
                break;
            }
        }
        self.bytes += snapshot.len();
        self.next = id;
        self.entries.push_back(Entry { info, snapshot });
        Ok(id)
    }
    pub fn infos(&self) -> Vec<BookmarkInfo> {
        self.entries.iter().map(|v| v.info.clone()).collect()
    }
    pub fn snapshot(&self, id: u64) -> Result<Vec<u8>> {
        Ok(self
            .entries
            .iter()
            .find(|v| v.info.id == id)
            .context("bookmark expired or does not exist")?
            .snapshot
            .clone())
    }
    pub fn rename(&mut self, id: u64, label: String, note: String) -> Result<()> {
        let entry = self
            .entries
            .iter_mut()
            .find(|v| v.info.id == id)
            .context("bookmark expired or does not exist")?;
        let mut info = entry.info.clone();
        info.label = label;
        info.note = note;
        info.validate()?;
        entry.info = info;
        Ok(())
    }
    pub fn remove(&mut self, id: u64) -> Result<()> {
        let index = self
            .entries
            .iter()
            .position(|v| v.info.id == id)
            .context("bookmark expired or does not exist")?;
        if let Some(old) = self.entries.remove(index) {
            self.bytes -= old.snapshot.len();
        }
        Ok(())
    }
    pub fn clear(&mut self) {
        self.entries.clear();
        self.bytes = 0;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn info() -> BookmarkInfo {
        BookmarkInfo {
            id: 1,
            label: "Bug".into(),
            note: String::new(),
            scene: "game".into(),
            frame: 4,
            time: 0.2,
            bytes: 0,
            profile: Default::default(),
        }
    }
    #[test]
    fn pinned_snapshots_are_bounded_and_evicted_ids_never_alias() {
        let mut store = Bookmarks::default();
        let first = store.record(info(), vec![7; 10]).unwrap();
        for _ in 0..8 {
            store.record(info(), vec![1; 10]).unwrap();
        }
        assert!(store.snapshot(first).is_err());
        assert_eq!(store.infos().len(), 8);
        let id = store.infos()[0].id;
        store.rename(id, "Fixed".into(), "Recheck".into()).unwrap();
        assert_eq!(store.infos()[0].label, "Fixed");
        store.remove(id).unwrap();
        assert!(store.snapshot(id).is_err());
        assert!(store.record(info(), vec![0; 16 * 1024 * 1024 + 1]).is_err());
        assert_eq!(store.infos().len(), 7);
    }
}
