//! Decoded sound caching and independently controlled playback voices.
use anyhow::{ensure, Context, Result};
use kairo_core::mixer::{BusStatus, BUSES};
use kairo_core::{ProjectFs, SoundHandle, VoiceHandle};
use kira::manager::{backend::DefaultBackend, AudioManager, AudioManagerSettings};
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::sound::PlaybackState;
use kira::track::{TrackBuilder, TrackHandle};
use kira::tween::Tween;
use std::collections::{BTreeMap, HashMap};
use std::io::Cursor;
use std::path::PathBuf;

const MAX_AUDIO_BYTES: usize = 256 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct PlaybackOptions {
    pub volume: f32,
    pub looping: bool,
    pub bus: String,
    pub pitch: f32,
    pub pan: f32,
}
impl Default for PlaybackOptions {
    fn default() -> Self {
        Self {
            volume: 1.0,
            looping: false,
            bus: "sfx".into(),
            pitch: 1.0,
            pan: 0.0,
        }
    }
}

pub struct AudioEngine {
    manager: Option<AudioManager<DefaultBackend>>,
    fs: ProjectFs,
    sounds: HashMap<SoundHandle, StaticSoundData>,
    paths: HashMap<PathBuf, SoundHandle>,
    voices: HashMap<VoiceHandle, StaticSoundHandle>,
    decoded_bytes: usize,
    tracks: HashMap<String, TrackHandle>,
    buses: BTreeMap<String, BusStatus>,
    voice_buses: HashMap<VoiceHandle, String>,
}

impl AudioEngine {
    pub fn new(fs: ProjectFs, enabled: bool) -> Self {
        let manager = if enabled {
            match AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()) {
                Ok(manager) => Some(manager),
                Err(error) => {
                    log::warn!(
                        "Audio unavailable: {error}. The game can query audio.isAvailable()."
                    );
                    None
                }
            }
        } else {
            None
        };
        Self {
            manager,
            fs,
            sounds: HashMap::new(),
            paths: HashMap::new(),
            voices: HashMap::new(),
            decoded_bytes: 0,
            tracks: HashMap::new(),
            buses: BUSES
                .iter()
                .map(|name| ((*name).to_owned(), BusStatus::new(name)))
                .collect(),
            voice_buses: HashMap::new(),
        }
    }

    pub fn available(&self) -> bool {
        self.manager.is_some()
    }

    pub fn load(&mut self, relative: &str) -> Result<SoundHandle> {
        let path = self.fs.resolve(relative)?;
        if let Some(handle) = self.paths.get(&path) {
            return Ok(*handle);
        }
        ensure!(self.sounds.len() < 1024, "sound cache limit reached (1024)");
        let bytes = self.fs.read(relative)?;
        let data = StaticSoundData::from_cursor(Cursor::new(bytes))
            .with_context(|| format!("cannot decode sound {relative}"))?;
        ensure!(
            data.sample_rate > 0 && !data.frames.is_empty(),
            "sound contains no playable samples"
        );
        let size = data.frames.len() * std::mem::size_of::<kira::Frame>();
        ensure!(
            self.decoded_bytes + size <= MAX_AUDIO_BYTES,
            "decoded audio cache exceeds 256 MiB"
        );
        let handle = SoundHandle::allocate()?;
        self.sounds.insert(handle, data);
        self.paths.insert(path, handle);
        self.decoded_bytes += size;
        log::info!("Loaded sound {relative}");
        Ok(handle)
    }

    pub fn duration(&self, sound: SoundHandle) -> Result<f64> {
        Ok(self
            .sounds
            .get(&sound)
            .context("unknown sound handle")?
            .duration()
            .as_secs_f64())
    }

    pub fn play(&mut self, sound: SoundHandle, volume: f32, looping: bool) -> Result<VoiceHandle> {
        self.play_with(
            sound,
            &PlaybackOptions {
                volume,
                looping,
                ..Default::default()
            },
        )
    }
    pub fn play_with(
        &mut self,
        sound: SoundHandle,
        options: &PlaybackOptions,
    ) -> Result<VoiceHandle> {
        validate_volume(options.volume)?;
        ensure!(
            options.pitch.is_finite() && (0.125..=4.0).contains(&options.pitch),
            "pitch must be 0.125..4"
        );
        ensure!(
            options.pan.is_finite() && (-1.0..=1.0).contains(&options.pan),
            "pan must be -1..1"
        );
        let bus = self
            .buses
            .get(&options.bus)
            .context("unknown audio bus")?
            .clone();
        self.prune();
        ensure!(self.voices.len() < 512, "voice limit reached (512)");
        let manager = self
            .manager
            .as_mut()
            .context("audio is disabled or no output device is available")?;
        if options.bus != "master" && !self.tracks.contains_key(&options.bus) {
            let mut track = manager
                .add_sub_track(TrackBuilder::new())
                .context("cannot create audio bus")?;
            track.set_volume(f64::from(bus.gain()), Tween::default());
            self.tracks.insert(options.bus.clone(), track);
        }
        let mut data = self
            .sounds
            .get(&sound)
            .context("unknown sound handle")?
            .volume(f64::from(options.volume))
            .playback_rate(f64::from(options.pitch))
            .panning(f64::from((options.pan + 1.0) * 0.5));
        if options.looping {
            data = data.loop_region(..);
        }
        if let Some(track) = self.tracks.get(&options.bus) {
            data = data.output_destination(track);
        }
        let id = VoiceHandle::allocate()?;
        let voice = manager.play(data).context("cannot start audio playback")?;
        self.voices.insert(id, voice);
        self.voice_buses.insert(id, options.bus.clone());
        Ok(id)
    }
    pub fn mixer(&self) -> Vec<BusStatus> {
        self.buses
            .values()
            .map(|bus| {
                let mut bus = bus.clone();
                bus.voices = self
                    .voice_buses
                    .values()
                    .filter(|name| bus.name == "master" || name.as_str() == bus.name.as_str())
                    .count();
                bus
            })
            .collect()
    }
    pub fn set_bus(
        &mut self,
        name: &str,
        volume: Option<f32>,
        muted: Option<bool>,
        seconds: f64,
    ) -> Result<()> {
        if let Some(volume) = volume {
            validate_volume(volume)?;
        }
        ensure!(
            seconds.is_finite() && (0.0..=30.0).contains(&seconds),
            "audio fade must be 0..30 seconds"
        );
        let bus = self
            .buses
            .get_mut(name)
            .context("unknown audio bus; choose master, music, sfx, ui")?;
        if let Some(volume) = volume {
            bus.volume = volume;
        }
        if let Some(muted) = muted {
            bus.muted = muted;
        }
        let tween = Tween {
            duration: std::time::Duration::from_secs_f64(seconds),
            ..Default::default()
        };
        if name == "master" {
            if let Some(manager) = &mut self.manager {
                manager
                    .main_track()
                    .set_volume(f64::from(bus.gain()), tween);
            }
        } else if let Some(track) = self.tracks.get_mut(name) {
            track.set_volume(f64::from(bus.gain()), tween);
        }
        Ok(())
    }
    pub fn stop_bus(&mut self, name: &str) -> Result<()> {
        ensure!(self.buses.contains_key(name), "unknown audio bus");
        let voices: Vec<_> = self
            .voice_buses
            .iter()
            .filter(|(_, bus)| name == "master" || bus.as_str() == name)
            .map(|(id, _)| *id)
            .collect();
        for voice in voices {
            self.stop(voice);
        }
        Ok(())
    }
    pub fn stop(&mut self, voice: VoiceHandle) {
        self.voice_buses.remove(&voice);
        if let Some(mut voice) = self.voices.remove(&voice) {
            voice.stop(Tween::default());
        }
    }

    pub fn stop_all(&mut self) {
        self.voice_buses.clear();
        for (_, mut voice) in self.voices.drain() {
            voice.stop(Tween::default());
        }
    }

    pub fn set_volume(&mut self, voice: VoiceHandle, volume: f32) -> Result<()> {
        validate_volume(volume)?;
        self.voices
            .get_mut(&voice)
            .context("voice is no longer playing")?
            .set_volume(f64::from(volume), Tween::default());
        Ok(())
    }

    pub fn set_master_volume(&mut self, volume: f32) -> Result<()> {
        validate_volume(volume)?;
        ensure!(self.manager.is_some(), "audio is unavailable");
        self.set_bus("master", Some(volume), None, 0.0)
    }

    pub fn is_playing(&self, voice: VoiceHandle) -> bool {
        self.voices
            .get(&voice)
            .is_some_and(|voice| voice.state() != PlaybackState::Stopped)
    }

    pub fn prune(&mut self) {
        self.voices
            .retain(|_, voice| voice.state() != PlaybackState::Stopped);
        self.voice_buses
            .retain(|id, _| self.voices.contains_key(id));
    }

    pub fn sound_count(&self) -> usize {
        self.sounds.len()
    }

    pub fn voice_count(&self) -> usize {
        self.voices.len()
    }

    pub fn decoded_bytes(&self) -> usize {
        self.decoded_bytes
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        self.stop_all();
    }
}

fn validate_volume(volume: f32) -> Result<()> {
    ensure!(
        volume.is_finite() && (0.0..=1.0).contains(&volume),
        "volume must be in 0..=1"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_audio_reports_unavailability_without_a_device() {
        let root = tempfile::tempdir().unwrap();
        let mut audio = AudioEngine::new(ProjectFs::new(root.path()).unwrap(), false);
        assert!(!audio.available());
        assert!(audio.set_master_volume(0.5).is_err());
        assert!(!audio.is_playing(VoiceHandle::allocate().unwrap()));
    }

    #[test]
    fn mixer_state_and_muting_are_testable_without_a_device() {
        let root = tempfile::tempdir().unwrap();
        let mut audio = AudioEngine::new(ProjectFs::new(root.path()).unwrap(), false);
        audio.set_bus("music", Some(0.4), Some(true), 0.2).unwrap();
        let music = audio
            .mixer()
            .into_iter()
            .find(|b| b.name == "music")
            .unwrap();
        assert_eq!(music.volume, 0.4);
        assert_eq!(music.gain(), 0.0);
        audio.set_bus("music", None, Some(false), 0.0).unwrap();
        assert!(audio.set_bus("bogus", None, None, 0.0).is_err());
        assert!(audio.set_bus("music", Some(2.0), None, 0.0).is_err());
        assert_eq!(
            audio
                .mixer()
                .into_iter()
                .find(|b| b.name == "music")
                .unwrap()
                .volume,
            0.4
        );
    }
    #[test]
    fn volumes_are_normalized_and_finite() {
        assert!(validate_volume(0.0).is_ok());
        assert!(validate_volume(1.0).is_ok());
        assert!(validate_volume(1.1).is_err());
        assert!(validate_volume(f32::NAN).is_err());
    }

    #[test]
    fn wav_is_decoded_and_cached_without_opening_an_audio_device() {
        let root = tempfile::tempdir().unwrap();
        // Four mono PCM samples in a complete little-endian RIFF container.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&44_u32.to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&8000_u32.to_le_bytes());
        bytes.extend_from_slice(&16000_u32.to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&8_u32.to_le_bytes());
        bytes.extend_from_slice(&[0; 8]);
        std::fs::write(root.path().join("tone.wav"), bytes).unwrap();
        let mut audio = AudioEngine::new(ProjectFs::new(root.path()).unwrap(), false);
        let a = audio.load("tone.wav").unwrap();
        assert_eq!(a, audio.load("./tone.wav").unwrap());
        assert_eq!(audio.sound_count(), 1);
        assert!(audio.duration(a).unwrap() > 0.0);
        assert!(audio.play(a, 1.0, false).is_err());
    }
}
