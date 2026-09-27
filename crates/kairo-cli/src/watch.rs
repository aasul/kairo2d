use anyhow::{Context, Result};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TrySendError};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct Changes {
    pub scripts: bool,
    pub textures: Vec<PathBuf>,
}

#[derive(Default)]
struct Pending {
    scripts: bool,
    textures: BTreeSet<PathBuf>,
    changed: Option<Instant>,
}

impl Pending {
    fn record(&mut self, path: &Path, now: Instant) {
        if path.components().any(|part| {
            matches!(
                part.as_os_str().to_str(),
                Some(".git" | ".github" | ".kairo" | "target" | "dist")
            )
        }) {
            return;
        }
        match path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("lua" | "json" | "tmj" | "tsj" | "ttf" | "otf") => self.scripts = true,
            Some("png" | "jpg" | "jpeg") => {
                if self.textures.len() < 4096 {
                    self.textures.insert(path.to_owned());
                } else {
                    self.scripts = true;
                }
            }
            _ => return,
        }
        self.changed = Some(now);
    }

    fn ready(&mut self, now: Instant) -> Option<Changes> {
        if !self
            .changed
            .is_some_and(|last| now.duration_since(last) >= Duration::from_millis(200))
        {
            return None;
        }
        self.changed = None;
        Some(Changes {
            scripts: std::mem::take(&mut self.scripts),
            textures: std::mem::take(&mut self.textures).into_iter().collect(),
        })
    }
}

pub struct ScriptWatcher {
    _watcher: RecommendedWatcher,
    receiver: Receiver<notify::Result<Event>>,
    root: PathBuf,
    pending: Pending,
    overflowed: Arc<AtomicBool>,
}

impl ScriptWatcher {
    pub fn new(root: &Path) -> Result<Self> {
        let (sender, receiver) = mpsc::sync_channel(4096);
        let overflowed = Arc::new(AtomicBool::new(false));
        let overflow_signal = overflowed.clone();
        let mut watcher = notify::recommended_watcher(move |event| {
            if matches!(sender.try_send(event), Err(TrySendError::Full(_))) {
                overflow_signal.store(true, Ordering::Relaxed);
            }
        })?;
        watcher
            .watch(root, RecursiveMode::Recursive)
            .context("cannot watch the game directory")?;
        Ok(Self {
            _watcher: watcher,
            receiver,
            root: root.to_owned(),
            pending: Pending::default(),
            overflowed,
        })
    }

    pub fn ready(&mut self) -> Option<Changes> {
        let now = Instant::now();
        if self.overflowed.swap(false, Ordering::Relaxed) {
            self.pending.scripts = true;
            self.pending.changed = Some(now);
            log::warn!("File watcher queue overflowed; scheduling a full session reload");
        }
        for result in self.receiver.try_iter().take(4096) {
            match result {
                Ok(event) if !matches!(event.kind, EventKind::Access(_)) => {
                    for path in event.paths {
                        if let Ok(relative) = path.strip_prefix(&self.root) {
                            self.pending.record(relative, now);
                        }
                    }
                }
                Err(error) => log::warn!("File watcher: {error}"),
                _ => {}
            }
        }
        self.pending.ready(now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_events_debounce_and_deduplicate() {
        let mut pending = Pending::default();
        let now = Instant::now();
        pending.record(Path::new("assets/player.png"), now);
        pending.record(
            Path::new("assets/player.png"),
            now + Duration::from_millis(100),
        );
        pending.record(
            Path::new("scripts/player.lua"),
            now + Duration::from_millis(100),
        );
        pending.record(
            Path::new("dist/game/main.lua"),
            now + Duration::from_millis(190),
        );
        assert!(pending.ready(now + Duration::from_millis(299)).is_none());
        let changes = pending.ready(now + Duration::from_millis(300)).unwrap();
        assert!(changes.scripts);
        assert_eq!(changes.textures, vec![PathBuf::from("assets/player.png")]);
        assert!(pending.ready(now + Duration::from_secs(1)).is_none());
    }
}
