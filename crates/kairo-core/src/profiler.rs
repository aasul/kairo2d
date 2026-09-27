use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::hash::{Hash, Hasher};
const MAX_CALLBACK_RECORDS: usize = 128;
const MAX_TELEMETRY_CALLBACKS: usize = 32;

fn bounded_label(value: &str, max_chars: usize) -> Cow<'_, str> {
    if value.len() <= max_chars {
        Cow::Borrowed(value)
    } else {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        value.hash(&mut hasher);
        let prefix: String = value.chars().take(max_chars.saturating_sub(17)).collect();
        Cow::Owned(format!("{prefix}~{:016x}", hasher.finish()))
    }
}

/// One callback's measurements in the latest simulation frame.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LuaCallbackSample {
    pub script: String,
    pub scene: String,
    pub node_path: String,
    pub callback: String,
    pub calls: u32,
    pub total_ms: f64,
    pub max_ms: f64,
}

impl LuaCallbackSample {
    pub fn average_ms(&self) -> f64 {
        self.total_ms / f64::from(self.calls.max(1))
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProfileSample {
    pub fps: f32,
    pub frame_ms: f64,
    #[serde(default)]
    pub tick_ms: f64,
    pub update_ms: f64,
    pub draw_ms: f64,
    pub physics_ms: f64,
    #[serde(default)]
    pub game_ms: f64,
    #[serde(default)]
    pub scene_ms: f64,
    #[serde(default)]
    pub ui_ms: f64,
    #[serde(default)]
    pub audio_ms: f64,
    #[serde(default)]
    pub render_ms: f64,
    #[serde(default)]
    pub profiling_enabled: bool,
    #[serde(default)]
    pub lua_callbacks: u32,
    #[serde(default)]
    pub callback_samples: Vec<LuaCallbackSample>,
    #[serde(default)]
    pub callback_samples_dropped: u32,
    pub commands: usize,
    pub batches: usize,
    pub vertices: usize,
    pub textures: usize,
    #[serde(default)]
    pub sprites_submitted: usize,
    #[serde(default)]
    pub active_bodies: usize,
    #[serde(default)]
    pub active_nodes: usize,
    #[serde(default)]
    pub active_colliders: usize,
    #[serde(default)]
    pub active_particles: usize,
    #[serde(default)]
    pub active_emitters: usize,
    #[serde(default)]
    pub audio_voices: usize,
    #[serde(default)]
    pub loaded_audio: usize,
    pub texture_bytes: usize,
    pub reloads: u64,
}

impl ProfileSample {
    pub fn begin_frame(&mut self) {
        self.lua_callbacks = 0;
        self.callback_samples.clear();
        self.callback_samples_dropped = 0;
        self.scene_ms = 0.0;
        self.game_ms = 0.0;
        self.ui_ms = 0.0;
        self.audio_ms = 0.0;
        self.render_ms = 0.0;
        self.tick_ms = 0.0;
    }

    /// Bounded collection keeps remote telemetry small even in large scenes.
    pub fn record_callback(
        &mut self,
        script: &str,
        scene: &str,
        node_path: &str,
        callback: &str,
        elapsed_ms: f64,
    ) {
        if !self.profiling_enabled {
            return;
        }
        let script = bounded_label(script, 64);
        let scene = bounded_label(scene, 32);
        let node_path = bounded_label(node_path, 96);
        let callback = bounded_label(callback, 32);
        self.lua_callbacks = self.lua_callbacks.saturating_add(1);
        if let Some(sample) = self.callback_samples.iter_mut().find(|sample| {
            sample.script == script.as_ref()
                && sample.scene == scene.as_ref()
                && sample.node_path == node_path.as_ref()
                && sample.callback == callback.as_ref()
        }) {
            sample.calls = sample.calls.saturating_add(1);
            sample.total_ms += elapsed_ms;
            sample.max_ms = sample.max_ms.max(elapsed_ms);
        } else if self.callback_samples.len() < MAX_CALLBACK_RECORDS {
            self.callback_samples.push(LuaCallbackSample {
                script: script.into_owned(),
                scene: scene.into_owned(),
                node_path: node_path.into_owned(),
                callback: callback.into_owned(),
                calls: 1,
                total_ms: elapsed_ms,
                max_ms: elapsed_ms,
            });
        } else {
            self.callback_samples_dropped = self.callback_samples_dropped.saturating_add(1);
        }
    }

    pub fn for_telemetry(&self) -> Self {
        let mut sample = self.clone();
        sample
            .callback_samples
            .sort_by(|a, b| b.total_ms.total_cmp(&a.total_ms));
        if sample.callback_samples.len() > MAX_TELEMETRY_CALLBACKS {
            sample.callback_samples_dropped = sample.callback_samples_dropped.saturating_add(
                sample.callback_samples[MAX_TELEMETRY_CALLBACKS..]
                    .iter()
                    .map(|entry| entry.calls)
                    .sum::<u32>(),
            );
            sample.callback_samples.truncate(MAX_TELEMETRY_CALLBACKS);
        }
        sample
    }
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
    RuntimePage {
        session: u64,
        offset: usize,
    },
    RuntimeSelect {
        session: u64,
        key: crate::inspector::RuntimeNodeKey,
    },
    RuntimeEdit {
        key: crate::inspector::RuntimeNodeKey,
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
        #[serde(default)]
        origins: bool,
        #[serde(default)]
        names: bool,
        #[serde(default)]
        camera: bool,
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
                p.tick_ms,
                p.update_ms,
                p.draw_ms,
                p.physics_ms,
                p.game_ms,
                p.scene_ms,
                p.ui_ms,
                p.audio_ms,
                p.render_ms,
                f64::from(p.fps)
            ]
            .iter()
            .all(|v| v.is_finite() && *v >= 0.0),
            "invalid remote profiler sample"
        );
        ensure!(
            p.callback_samples.len() <= MAX_TELEMETRY_CALLBACKS,
            "too many callback samples"
        );
        for sample in &p.callback_samples {
            ensure!(
                sample.script.len() <= 256
                    && sample.scene.len() <= 128
                    && sample.node_path.len() <= 384
                    && sample.callback.len() <= 128
                    && sample.calls > 0
                    && sample.total_ms.is_finite()
                    && sample.total_ms >= 0.0
                    && sample.max_ms.is_finite()
                    && sample.max_ms >= 0.0
                    && sample.max_ms <= sample.total_ms,
                "invalid Lua callback sample"
            );
        }
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
            Self::RuntimePage { offset, .. } => {
                ensure!(*offset <= 50_000, "runtime page is out of range")
            }
            Self::RuntimeSelect { key, .. } => {
                ensure!(key.graph > 0 && key.id > 0, "invalid runtime node identity")
            }
            Self::RuntimeEdit { key, update } => {
                ensure!(key.graph > 0 && key.id > 0, "invalid runtime node identity");
                ensure!(
                    !update.path.is_empty() && update.path.len() <= 128,
                    "invalid runtime field"
                );
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callbacks_aggregate_and_reset_at_frame_boundary() {
        let mut p = ProfileSample {
            profiling_enabled: true,
            ..Default::default()
        };
        p.record_callback("scripts/enemy.lua", "arena", "arena/enemy", "update", 1.0);
        p.record_callback("scripts/enemy.lua", "arena", "arena/enemy", "update", 3.0);
        assert_eq!(p.lua_callbacks, 2);
        assert_eq!(p.callback_samples.len(), 1);
        assert_eq!(p.callback_samples[0].calls, 2);
        assert_eq!(p.callback_samples[0].average_ms(), 2.0);
        assert_eq!(p.callback_samples[0].max_ms, 3.0);
        p.begin_frame();
        assert_eq!(p.lua_callbacks, 0);
        assert!(p.callback_samples.is_empty());
        p.profiling_enabled = false;
        p.record_callback("scripts/enemy.lua", "arena", "arena/enemy", "update", 1.0);
        assert_eq!(p.lua_callbacks, 0);
        p.profiling_enabled = true;
        let long_path = "enemy/".repeat(40);
        p.record_callback("scripts/enemy.lua", "arena", &long_path, "update", 1.0);
        p.record_callback("scripts/enemy.lua", "arena", &long_path, "update", 2.0);
        assert_eq!(p.callback_samples.len(), 1);
        assert_eq!(p.callback_samples[0].calls, 2);
        p.record_callback(
            "scripts/enemy.lua",
            "arena",
            &(long_path + "other"),
            "update",
            1.0,
        );
        assert_eq!(p.callback_samples.len(), 2);
    }

    #[test]
    fn callback_telemetry_is_bounded_and_rejects_invalid_timings() {
        let mut telemetry = Telemetry::default();
        telemetry.profile.profiling_enabled = true;
        for index in 0..80 {
            telemetry
                .profile
                .record_callback("main.lua", "", "", &format!("cb{index}"), 0.1);
        }
        assert_eq!(telemetry.profile.callback_samples.len(), 80);
        telemetry.profile = telemetry.profile.for_telemetry();
        assert_eq!(telemetry.profile.callback_samples.len(), 32);
        assert_eq!(telemetry.profile.callback_samples_dropped, 48);
        telemetry.validate().unwrap();
        telemetry.profile.callback_samples[0].total_ms = f64::NAN;
        assert!(telemetry.validate().is_err());
    }
}
