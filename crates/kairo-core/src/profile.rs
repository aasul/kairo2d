use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildProfile {
    #[default]
    Development,
    Debug,
    Release,
    Micro,
}
impl BuildProfile {
    pub const ALL: [Self; 4] = [Self::Development, Self::Debug, Self::Release, Self::Micro];
    pub fn name(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Debug => "debug",
            Self::Release => "release",
            Self::Micro => "micro",
        }
    }
    pub fn parse(name: &str) -> Result<Self> {
        Self::ALL
            .into_iter()
            .find(|p| p.name() == name)
            .ok_or_else(|| {
                anyhow::anyhow!("unknown profile; choose development, debug, release, or micro")
            })
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProfileOverrides {
    pub hot_reload: Option<bool>,
    pub tools: Option<bool>,
    pub profiler: Option<bool>,
    pub link: Option<bool>,
    pub replay: Option<bool>,
    pub micro: Option<bool>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub vsync: Option<bool>,
    pub log_level: Option<String>,
}
impl ProfileOverrides {
    pub fn validate(&self) -> Result<()> {
        for value in [self.width, self.height].into_iter().flatten() {
            ensure!(
                (1..=8192).contains(&value),
                "profile window size must be 1..8192"
            );
        }
        ensure!(
            self.log_level
                .as_ref()
                .is_none_or(
                    |v| ["error", "warn", "info", "debug", "trace", "off"].contains(&v.as_str())
                ),
            "invalid profile log_level"
        );
        Ok(())
    }
}

impl crate::Config {
    pub fn load_profile(fs: &crate::ProjectFs, profile: BuildProfile) -> Result<Self> {
        let mut config = Self::load(fs)?;
        config.apply_profile(profile)?;
        Ok(config)
    }
    pub fn apply_profile(&mut self, profile: BuildProfile) -> Result<()> {
        if profile == BuildProfile::Release {
            self.development.hot_reload = false;
            self.development.tools = false;
            self.development.link = false;
            self.development.profiler = false;
            self.development.log_level = "warn".into();
            self.replay.enabled = false;
        } else if profile == BuildProfile::Debug {
            self.development.profiler = true;
        } else if profile == BuildProfile::Micro {
            self.micro.enabled = true;
        }
        if let Some(overrides) = self.profile.get(profile.name()) {
            overrides.validate()?;
            if let Some(v) = overrides.hot_reload {
                self.development.hot_reload = v;
            }
            if let Some(v) = overrides.tools {
                self.development.tools = v;
            }
            if let Some(v) = overrides.link {
                self.development.link = v;
            }
            if let Some(v) = overrides.profiler {
                self.development.profiler = v;
            }
            if let Some(v) = overrides.replay {
                self.replay.enabled = v;
            }
            if let Some(v) = overrides.micro {
                self.micro.enabled = v;
            }
            if let Some(v) = overrides.width {
                self.game.width = v;
            }
            if let Some(v) = overrides.height {
                self.game.height = v;
            }
            if let Some(v) = overrides.vsync {
                self.game.vsync = v;
            }
            if let Some(v) = &overrides.log_level {
                self.development.log_level.clone_from(v);
            }
        }
        if !self.development.tools {
            self.development.profiler = false;
        }
        self.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn release_disables_development_channels_but_accepts_explicit_overrides() {
        let mut config = crate::Config::default();
        config.replay.enabled = true;
        config.apply_profile(BuildProfile::Release).unwrap();
        assert!(
            !config.development.hot_reload
                && !config.development.tools
                && !config.development.link
                && !config.replay.enabled
        );
        let mut config =
            crate::Config::parse("[profile.release]\ntools=true\nprofiler=true\nwidth=800")
                .unwrap();
        config.apply_profile(BuildProfile::Release).unwrap();
        assert!(config.development.tools && config.development.profiler);
        assert_eq!(config.game.width, 800);
        assert!(crate::Config::parse("[profile.release]\nwidth=0").is_err());
    }
    #[test]
    fn micro_changes_render_profile_not_physical_window_or_compiler() {
        let mut config = crate::Config::default();
        config.apply_profile(BuildProfile::Micro).unwrap();
        assert!(config.micro.enabled);
        assert_eq!(config.game.width, 960);
        assert_eq!(config.micro.width, 320);
        assert!(BuildProfile::parse("optimized-magic").is_err());
    }
}
