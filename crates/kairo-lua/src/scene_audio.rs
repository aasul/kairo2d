//! Scene AudioSource playback through the existing Kira-backed audio manager.
use anyhow::{ensure, Context, Result};
use kairo_audio::{AudioEngine, PlaybackOptions};
use kairo_core::scene_graph::{NodeFile, NodeId, NodeKind, SceneGraph};
use kairo_core::VoiceHandle;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq)]
struct SourceSpec {
    path: String,
    volume: f32,
    looping: bool,
    bus: String,
}

struct SourceEntry {
    spec: SourceSpec,
    voice: Option<VoiceHandle>,
}

#[derive(Default)]
pub(crate) struct SceneAudio {
    sources: HashMap<NodeId, SourceEntry>,
}

fn source(node: &NodeFile) -> Result<Option<SourceSpec>> {
    let autoplay = match node.properties.get("autoplay") {
        None => false,
        Some(Value::Bool(value)) => *value,
        _ => anyhow::bail!("autoplay must be a boolean"),
    };
    if !autoplay {
        return Ok(None);
    }
    let path = node
        .properties
        .get("sound")
        .and_then(Value::as_str)
        .context("AudioSource autoplay needs a sound path")?;
    ensure!(
        path.ends_with(".wav") || path.ends_with(".ogg"),
        "AudioSource sound must be a .wav or .ogg project path"
    );
    let volume = match node.properties.get("volume") {
        None => 1.0,
        Some(Value::Number(value)) => value.as_f64().context("invalid volume")? as f32,
        _ => anyhow::bail!("volume must be a number"),
    };
    ensure!(
        volume.is_finite() && (0.0..=1.0).contains(&volume),
        "volume must be in 0..=1"
    );
    let looping = match node.properties.get("looping") {
        None => false,
        Some(Value::Bool(value)) => *value,
        _ => anyhow::bail!("looping must be a boolean"),
    };
    let bus = match node.properties.get("bus") {
        None => "sfx",
        Some(Value::String(value)) => value.as_str(),
        _ => anyhow::bail!("bus must be a string"),
    };
    ensure!(kairo_core::mixer::BUSES.contains(&bus), "unknown audio bus");
    Ok(Some(SourceSpec {
        path: path.to_owned(),
        volume,
        looping,
        bus: bus.to_owned(),
    }))
}

impl SceneAudio {
    pub(crate) fn sync(&mut self, graph: &SceneGraph, audio: &mut AudioEngine) -> Result<()> {
        let mut pending = vec![graph.root()];
        let mut desired = Vec::new();
        while let Some(id) = pending.pop() {
            let node = graph.node(id)?;
            if !node.data().enabled {
                continue;
            }
            pending.extend(node.children().iter().rev());
            if node.data().kind == NodeKind::AudioSource {
                if let Some(spec) = source(node.data()).with_context(|| {
                    format!(
                        "scene '{}', node '{}': native audio",
                        graph.name(),
                        graph.path(id).unwrap_or_default()
                    )
                })? {
                    desired.push((id, spec));
                }
            }
        }
        let keep: HashSet<_> = desired.iter().map(|entry| entry.0).collect();
        for id in self.sources.keys().copied().collect::<Vec<_>>() {
            if !keep.contains(&id) {
                if let Some(voice) = self.sources.remove(&id).and_then(|entry| entry.voice) {
                    audio.stop(voice);
                }
            }
        }
        for (id, spec) in desired {
            if self
                .sources
                .get(&id)
                .is_some_and(|entry| entry.spec == spec)
            {
                continue;
            }
            if let Some(voice) = self.sources.remove(&id).and_then(|entry| entry.voice) {
                audio.stop(voice);
            }
            let sound = audio.load(&spec.path).with_context(|| {
                format!(
                    "scene '{}', node '{}': load sound",
                    graph.name(),
                    graph.path(id).unwrap_or_default()
                )
            })?;
            let voice = if audio.available() {
                Some(audio.play_with(
                    sound,
                    &PlaybackOptions {
                        volume: spec.volume,
                        looping: spec.looping,
                        bus: spec.bus.clone(),
                        ..Default::default()
                    },
                )?)
            } else {
                None
            };
            self.sources.insert(id, SourceEntry { spec, voice });
        }
        Ok(())
    }

    pub(crate) fn deactivate(&mut self, audio: &mut AudioEngine) {
        for (_, entry) in self.sources.drain() {
            if let Some(voice) = entry.voice {
                audio.stop(voice);
            }
        }
    }
}
