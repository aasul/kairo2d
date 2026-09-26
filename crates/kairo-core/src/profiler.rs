use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProfileSample {
    pub fps: f32,
    pub frame_ms: f64,
    pub update_ms: f64,
    pub draw_ms: f64,
    pub physics_ms: f64,
    pub commands: usize,
    pub batches: usize,
    pub vertices: usize,
    pub textures: usize,
    pub texture_bytes: usize,
    pub reloads: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ReplayStatus {
    pub recording: bool,
    pub paused: bool,
    pub frame: u64,
    pub time: f64,
    pub oldest: f64,
    pub newest: f64,
    pub snapshots: usize,
    pub bytes: usize,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Telemetry {
    pub profile: ProfileSample,
    #[serde(default)]
    pub tools_enabled: bool,
    #[serde(default)]
    pub bookmarks: Vec<crate::bookmarks::BookmarkInfo>,
    #[serde(default)]
    pub debug_draw: crate::debug_draw::DebugDraw,
    pub replay: ReplayStatus,
    #[serde(default)]
    pub mixer: Vec<crate::mixer::BusStatus>,
    #[serde(default)]
    pub inspection: crate::inspector::InspectSnapshot,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum DebugCommand {
    Pause,
    Resume,
    Step,
    Seek {
        time: f64,
    },
    Record {
        enabled: bool,
    },
    Inspect {
        update: Box<crate::inspector::InspectUpdate>,
    },
    Scene {
        session: u64,
        operation: SceneOperation,
        name: Option<String>,
    },
    DebugDraw {
        physics: bool,
        bounds: bool,
        velocities: bool,
    },
    Mixer {
        bus: String,
        volume: Option<f32>,
        muted: Option<bool>,
        fade: f64,
        stop: bool,
    },
    Language {
        language: String,
    },
    Bookmark {
        label: String,
        note: String,
    },
    BookmarkJump {
        session: u64,
        id: u64,
    },
    BookmarkEdit {
        session: u64,
        id: u64,
        label: String,
        note: String,
    },
    BookmarkDelete {
        session: u64,
        id: u64,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneOperation {
    Switch,
    Push,
    Pop,
    Reload,
}

impl Telemetry {
    pub fn validate(&self) -> anyhow::Result<()> {
        use anyhow::ensure;
        self.inspection.validate()?;
        ensure!(
            self.bookmarks.len() <= 8 && self.mixer.len() <= 4,
            "telemetry collection limit exceeded"
        );
        for bookmark in &self.bookmarks {
            bookmark.validate()?;
        }
        for bus in &self.mixer {
            ensure!(
                crate::mixer::BUSES.contains(&bus.name.as_str())
                    && bus.volume.is_finite()
                    && (0.0..=1.0).contains(&bus.volume),
                "invalid mixer telemetry"
            );
        }
        let p = &self.profile;
        ensure!(
            [
                p.frame_ms,
                p.update_ms,
                p.draw_ms,
                p.physics_ms,
                f64::from(p.fps)
            ]
            .iter()
            .all(|v| v.is_finite() && *v >= 0.0),
            "invalid remote profiler sample"
        );
        ensure!(
            [self.replay.time, self.replay.oldest, self.replay.newest]
                .iter()
                .all(|v| v.is_finite() && *v >= 0.0),
            "invalid remote replay sample"
        );
        ensure!(
            serde_json::to_vec(self)?.len() <= 40 * 1024,
            "telemetry exceeds 40 KiB"
        );
        Ok(())
    }
}
impl DebugCommand {
    pub fn validate(&self) -> anyhow::Result<()> {
        use anyhow::ensure;
        ensure!(
            serde_json::to_vec(self)?.len() <= 4096,
            "debug command exceeds 4 KiB"
        );
        match self {
            Self::Seek { time } => ensure!(time.is_finite() && *time >= 0.0, "invalid replay seek"),
            Self::Scene {
                name, operation, ..
            } => {
                let named = matches!(operation, SceneOperation::Switch | SceneOperation::Push);
                ensure!(
                    !named
                        || name
                            .as_ref()
                            .is_some_and(|n| !n.is_empty() && n.len() <= 64),
                    "scene command requires a name"
                );
                ensure!(
                    name.as_ref().is_none_or(|n| n.len() <= 64),
                    "scene name exceeds limit"
                );
            }
            Self::Inspect { update } => ensure!(
                !update.path.is_empty() && update.path.len() <= 128,
                "invalid inspector path"
            ),
            Self::Mixer {
                bus, volume, fade, ..
            } => {
                ensure!(
                    crate::mixer::BUSES.contains(&bus.as_str())
                        && volume.is_none_or(|v| v.is_finite() && (0.0..=1.0).contains(&v)),
                    "invalid mixer bus/volume"
                );
                ensure!(
                    fade.is_finite() && (0.0..=30.0).contains(fade),
                    "invalid mixer fade"
                );
            }
            Self::Language { language } => ensure!(
                !language.is_empty()
                    && language.len() <= 32
                    && language
                        .bytes()
                        .all(|v| v.is_ascii_alphanumeric() || v == b'_' || v == b'-'),
                "invalid language identifier"
            ),
            Self::Bookmark { label, note } | Self::BookmarkEdit { label, note, .. } => ensure!(
                !label.is_empty() && label.len() <= 96 && note.len() <= 512,
                "invalid bookmark label/note"
            ),
            _ => {}
        }
        Ok(())
    }
}
