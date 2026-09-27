use crate::ProjectFs;
use anyhow::{ensure, Context, Result};
use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub game: GameConfig,
    pub audio: AudioConfig,
    pub development: DevelopmentConfig,
    pub physics: PhysicsConfig,
    pub save: SaveConfig,
    pub replay: ReplayConfig,
    pub micro: MicroConfig,
    pub profile: std::collections::BTreeMap<String, crate::profile::ProfileOverrides>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(default)]
pub struct MicroConfig {
    pub enabled: bool,
    pub width: u32,
    pub height: u32,
    pub integer_scaling: bool,
    pub pixel_snap: bool,
    pub palette: String,
}
impl Default for MicroConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            width: 320,
            height: 180,
            integer_scaling: true,
            pixel_snap: true,
            palette: "kairo16".into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct ReplayConfig {
    pub enabled: bool,
    pub seconds: f64,
    pub frequency: u32,
    pub memory_mib: usize,
}
impl Default for ReplayConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            seconds: 10.0,
            frequency: 30,
            memory_mib: 32,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct SaveConfig {
    pub identity: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct GameConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub vsync: bool,
    pub resizable: bool,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            title: "Kairo2D".into(),
            width: 960,
            height: 540,
            vsync: true,
            resizable: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct AudioConfig {
    pub enabled: bool,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct DevelopmentConfig {
    pub hot_reload: bool,
    pub tools: bool,
    pub profiler: bool,
    pub link: bool,
    pub log_level: String,
}

impl Default for DevelopmentConfig {
    fn default() -> Self {
        Self {
            hot_reload: true,
            tools: true,
            profiler: false,
            link: true,
            log_level: "info".into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct PhysicsConfig {
    pub gravity_x: f32,
    pub gravity_y: f32,
    pub pixels_per_meter: f32,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            gravity_x: 0.0,
            gravity_y: 980.0,
            pixels_per_meter: 100.0,
        }
    }
}

impl Config {
    pub fn parse(source: &str) -> Result<Self> {
        let config: Self = toml::from_str(source).context("invalid kairo.toml")?;
        config.validate()?;
        Ok(config)
    }

    pub fn load(fs: &ProjectFs) -> Result<Self> {
        if !fs.exists("kairo.toml")? {
            return Ok(Self::default());
        }
        Self::parse(&fs.read_text("kairo.toml")?)
    }

    pub fn validate(&self) -> Result<()> {
        for (name, profile) in &self.profile {
            crate::profile::BuildProfile::parse(name)?;
            profile.validate()?;
        }
        ensure!(
            ["error", "warn", "info", "debug", "trace", "off"]
                .contains(&self.development.log_level.as_str()),
            "invalid development.log_level"
        );
        ensure!(
            (1..=8192).contains(&self.game.width),
            "game.width must be in 1..=8192"
        );
        ensure!(
            (1..=8192).contains(&self.game.height),
            "game.height must be in 1..=8192"
        );
        ensure!(
            !self.game.title.trim().is_empty(),
            "game.title cannot be empty"
        );
        ensure!(
            self.game.title.len() <= 256,
            "game.title must be at most 256 bytes"
        );
        ensure!(
            (1..=2048).contains(&self.micro.width) && (1..=2048).contains(&self.micro.height),
            "Micro canvas dimensions must be 1..=2048"
        );
        crate::micro::palette(&self.micro.palette)?;
        ensure!(
            self.save.identity.is_empty()
                || (self.save.identity.len() <= 96
                    && self
                        .save
                        .identity
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))),
            "save.identity must be empty or 1..=96 ASCII letters, digits, '-' or '_'"
        );
        ensure!(
            self.replay.seconds.is_finite() && (0.1..=120.0).contains(&self.replay.seconds),
            "replay.seconds must be 0.1..=120"
        );
        ensure!(
            (1..=120).contains(&self.replay.frequency)
                && (1..=256).contains(&self.replay.memory_mib),
            "invalid replay frequency/memory budget"
        );
        crate::finite("gravity", &[self.physics.gravity_x, self.physics.gravity_y])?;
        ensure!(
            self.physics.pixels_per_meter.is_finite() && self.physics.pixels_per_meter > 0.0,
            "physics.pixels_per_meter must be finite and positive"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omitted_fields_have_documented_defaults() {
        let c = Config::parse("[game]\ntitle = 'Test'").unwrap();
        assert_eq!(c.game.width, 960);
        assert!(c.game.vsync);
        assert_eq!(c.physics.pixels_per_meter, 100.0);
    }

    #[test]
    fn rejects_invalid_dimensions_and_nonfinite_physics() {
        assert!(Config::parse("[game]\nwidth = 0").is_err());
        assert!(Config::parse("[physics]\npixels_per_meter = 0").is_err());
        assert!(Config::parse("[physics]\ngravity_x = nan").is_err());
    }

    #[test]
    fn missing_config_is_optional_but_bad_config_is_not() {
        let root = tempfile::tempdir().unwrap();
        let fs = ProjectFs::new(root.path()).unwrap();
        assert!(Config::load(&fs).is_ok());
        std::fs::write(root.path().join("kairo.toml"), "[game\n").unwrap();
        assert!(Config::load(&fs).is_err());
    }
}
