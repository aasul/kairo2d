use crate::{ProjectFiles, SettingsDocument};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub enum Template {
    Scenes,
    InputActions,
    Inspector,
    Particles,
    Mixer,
    ReplayBugs,
    TopDown,
    Platformer,

    #[default]
    Empty,
    HelloWorld,
    Movement,
    Breakout,
    Micro,
    Ui,
    Link,
    Replay,
}

impl Template {
    pub const ALL: [Self; 16] = [
        Self::Scenes,
        Self::InputActions,
        Self::Inspector,
        Self::Particles,
        Self::Mixer,
        Self::ReplayBugs,
        Self::TopDown,
        Self::Platformer,
        Self::Empty,
        Self::HelloWorld,
        Self::Movement,
        Self::Breakout,
        Self::Micro,
        Self::Ui,
        Self::Link,
        Self::Replay,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Scenes => "Scene Demo",
            Self::InputActions => "Input Mapping Demo",
            Self::Inspector => "Live Inspector Demo",
            Self::Particles => "Particle Demo",
            Self::Mixer => "Audio Mixer Demo",
            Self::ReplayBugs => "Replay Bug Demo",
            Self::TopDown => "Top-Down Starter",
            Self::Platformer => "Platformer Starter",

            Self::Empty => "Empty Project",
            Self::HelloWorld => "Hello World",
            Self::Movement => "Basic Movement",
            Self::Breakout => "Breakout",
            Self::Micro => "Kairo Micro",
            Self::Ui => "UI Workshop",
            Self::Link => "Kairo Link Demo",
            Self::Replay => "Replay Debug Demo",
        }
    }

    pub fn parse(name: &str) -> Result<Self> {
        match name {
            "scenes" => Ok(Self::Scenes),
            "input-actions" => Ok(Self::InputActions),
            "live-inspector" => Ok(Self::Inspector),
            "particles" => Ok(Self::Particles),
            "audio-mixer" => Ok(Self::Mixer),
            "replay-bugs" => Ok(Self::ReplayBugs),
            "top-down" => Ok(Self::TopDown),
            "platformer" => Ok(Self::Platformer),

            "empty" => Ok(Self::Empty),
            "hello-world" => Ok(Self::HelloWorld),
            "movement" => Ok(Self::Movement),
            "breakout" => Ok(Self::Breakout),
            "micro" => Ok(Self::Micro),
            "ui" => Ok(Self::Ui),
            "link" => Ok(Self::Link),
            "replay" => Ok(Self::Replay),
            _ => anyhow::bail!("unknown template; see docs/cli.md for template identifiers"),
        }
    }

    fn files(self) -> &'static [(&'static str, &'static [u8])] {
        match self {
            Self::Scenes => &[
                ("README.md", include_bytes!("../templates/scenes/README.md")),
                (
                    "kairo.toml",
                    include_bytes!("../templates/scenes/kairo.toml"),
                ),
                ("main.lua", include_bytes!("../templates/scenes/main.lua")),
                (
                    "scenes/play.scene",
                    include_bytes!("../templates/scenes/scenes/play.scene"),
                ),
                (
                    "prefabs/marker.prefab",
                    include_bytes!("../templates/scenes/prefabs/marker.prefab"),
                ),
                (
                    "scripts/player.lua",
                    include_bytes!("../templates/scenes/scripts/player.lua"),
                ),
                (
                    "scripts/marker.lua",
                    include_bytes!("../templates/scenes/scripts/marker.lua"),
                ),
                (
                    "assets/player.png",
                    include_bytes!("../templates/scenes/assets/player.png"),
                ),
            ],
            Self::InputActions => &[
                (
                    "README.md",
                    include_bytes!("../templates/input-actions/README.md"),
                ),
                (
                    "input.toml",
                    include_bytes!("../templates/input-actions/input.toml"),
                ),
                (
                    "kairo.toml",
                    include_bytes!("../templates/input-actions/kairo.toml"),
                ),
                (
                    "main.lua",
                    include_bytes!("../templates/input-actions/main.lua"),
                ),
            ],
            Self::Inspector => &[
                (
                    "README.md",
                    include_bytes!("../templates/live-inspector/README.md"),
                ),
                (
                    "kairo.toml",
                    include_bytes!("../templates/live-inspector/kairo.toml"),
                ),
                (
                    "main.lua",
                    include_bytes!("../templates/live-inspector/main.lua"),
                ),
            ],
            Self::Particles => &[
                (
                    "README.md",
                    include_bytes!("../templates/particles/README.md"),
                ),
                (
                    "effects/sparks.particle.toml",
                    include_bytes!("../templates/particles/effects/sparks.particle.toml"),
                ),
                (
                    "kairo.toml",
                    include_bytes!("../templates/particles/kairo.toml"),
                ),
                (
                    "main.lua",
                    include_bytes!("../templates/particles/main.lua"),
                ),
            ],
            Self::Mixer => &[
                (
                    "README.md",
                    include_bytes!("../templates/audio-mixer/README.md"),
                ),
                (
                    "assets/tone.wav",
                    include_bytes!("../templates/audio-mixer/assets/tone.wav"),
                ),
                (
                    "kairo.toml",
                    include_bytes!("../templates/audio-mixer/kairo.toml"),
                ),
                (
                    "main.lua",
                    include_bytes!("../templates/audio-mixer/main.lua"),
                ),
            ],
            Self::ReplayBugs => &[
                (
                    "README.md",
                    include_bytes!("../templates/replay-bugs/README.md"),
                ),
                (
                    "kairo.toml",
                    include_bytes!("../templates/replay-bugs/kairo.toml"),
                ),
                (
                    "main.lua",
                    include_bytes!("../templates/replay-bugs/main.lua"),
                ),
            ],
            Self::TopDown => &[
                (
                    "README.md",
                    include_bytes!("../templates/top-down/README.md"),
                ),
                (
                    "assets/pickup.wav",
                    include_bytes!("../templates/top-down/assets/pickup.wav"),
                ),
                (
                    "assets/robot.png",
                    include_bytes!("../templates/top-down/assets/robot.png"),
                ),
                (
                    "assets/tiles.png",
                    include_bytes!("../templates/top-down/assets/tiles.png"),
                ),
                (
                    "assets/yard.tmj",
                    include_bytes!("../templates/top-down/assets/yard.tmj"),
                ),
                (
                    "effects/pickup.particle.toml",
                    include_bytes!("../templates/top-down/effects/pickup.particle.toml"),
                ),
                (
                    "input.toml",
                    include_bytes!("../templates/top-down/input.toml"),
                ),
                (
                    "kairo.toml",
                    include_bytes!("../templates/top-down/kairo.toml"),
                ),
                (
                    "locales/en.json",
                    include_bytes!("../templates/top-down/locales/en.json"),
                ),
                (
                    "locales/fr.json",
                    include_bytes!("../templates/top-down/locales/fr.json"),
                ),
                ("main.lua", include_bytes!("../templates/top-down/main.lua")),
                (
                    "prefabs/charge.toml",
                    include_bytes!("../templates/top-down/prefabs/charge.toml"),
                ),
            ],
            Self::Platformer => &[
                (
                    "README.md",
                    include_bytes!("../templates/platformer/README.md"),
                ),
                (
                    "kairo.toml",
                    include_bytes!("../templates/platformer/kairo.toml"),
                ),
                (
                    "main.lua",
                    include_bytes!("../templates/platformer/main.lua"),
                ),
            ],
            Self::Micro => &[
                ("README.md", include_bytes!("../templates/micro/README.md")),
                (
                    "kairo.toml",
                    include_bytes!("../templates/micro/kairo.toml"),
                ),
                ("main.lua", include_bytes!("../templates/micro/main.lua")),
            ],
            Self::Ui => &[
                ("README.md", include_bytes!("../templates/ui/README.md")),
                ("kairo.toml", include_bytes!("../templates/ui/kairo.toml")),
                ("main.lua", include_bytes!("../templates/ui/main.lua")),
            ],
            Self::Link => &[
                ("README.md", include_bytes!("../templates/link/README.md")),
                ("kairo.toml", include_bytes!("../templates/link/kairo.toml")),
                ("main.lua", include_bytes!("../templates/link/main.lua")),
            ],
            Self::Replay => &[
                ("README.md", include_bytes!("../templates/replay/README.md")),
                (
                    "kairo.toml",
                    include_bytes!("../templates/replay/kairo.toml"),
                ),
                ("main.lua", include_bytes!("../templates/replay/main.lua")),
            ],
            Self::Empty => &[
                (
                    "kairo.toml",
                    include_bytes!("../templates/empty/kairo.toml"),
                ),
                ("main.lua", include_bytes!("../templates/empty/main.lua")),
            ],
            Self::HelloWorld => &[
                (
                    "kairo.toml",
                    include_bytes!("../templates/hello-world/kairo.toml"),
                ),
                (
                    "main.lua",
                    include_bytes!("../templates/hello-world/main.lua"),
                ),
            ],
            Self::Movement => &[
                (
                    "assets/player.png",
                    include_bytes!("../templates/movement/assets/player.png"),
                ),
                (
                    "kairo.toml",
                    include_bytes!("../templates/movement/kairo.toml"),
                ),
                ("main.lua", include_bytes!("../templates/movement/main.lua")),
                (
                    "player.lua",
                    include_bytes!("../templates/movement/player.lua"),
                ),
            ],
            Self::Breakout => &[
                (
                    "README.md",
                    include_bytes!("../templates/breakout/README.md"),
                ),
                (
                    "assets/ball.png",
                    include_bytes!("../templates/breakout/assets/ball.png"),
                ),
                (
                    "assets/bounce.wav",
                    include_bytes!("../templates/breakout/assets/bounce.wav"),
                ),
                (
                    "collision.lua",
                    include_bytes!("../templates/breakout/collision.lua"),
                ),
                (
                    "kairo.toml",
                    include_bytes!("../templates/breakout/kairo.toml"),
                ),
                ("main.lua", include_bytes!("../templates/breakout/main.lua")),
            ],
        }
    }
}

pub fn create_project(path: &Path, title: &str, template: Template) -> Result<ProjectFiles> {
    ensure!(
        !title.trim().is_empty() && title.len() <= 256,
        "project title must contain 1 to 256 bytes"
    );
    std::fs::create_dir(path).context("choose a new project directory in an existing parent")?;
    let result = (|| -> Result<ProjectFiles> {
        for (relative, bytes) in template.files() {
            let destination = path.join(relative);
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(destination, bytes)?;
        }
        std::fs::create_dir_all(path.join("assets"))?;
        let files = ProjectFiles::open(path)?;
        let mut settings = SettingsDocument::load(&files)?;
        settings.config.game.title = title.to_owned();
        let mut identity = [0; 12];
        getrandom::fill(&mut identity)
            .map_err(|e| anyhow::anyhow!("cannot create project identity: {e}"))?;
        settings.config.save.identity = format!(
            "game-{}",
            identity
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        settings.save(&files)?;
        Ok(files)
    })();
    if result.is_err() {
        // create_dir above established exclusive ownership of this new directory.
        let _ = std::fs::remove_dir_all(path);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use kairo_core::Config;

    #[test]
    fn all_templates_create_valid_configuration_and_entrypoints() {
        let root = tempfile::tempdir().unwrap();
        for template in Template::ALL {
            let path = root.path().join(template.name());
            let files = create_project(&path, "A \"quoted\" game", template).unwrap();
            assert!(path.join("main.lua").is_file());
            assert!(path.join("assets").is_dir());
            assert_eq!(
                Config::load(files.filesystem()).unwrap().game.title,
                "A \"quoted\" game"
            );
            assert!(create_project(&path, "No overwrite", template).is_err());
        }
    }
}
