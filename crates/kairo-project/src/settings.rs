use crate::ProjectFiles;
use anyhow::{ensure, Context, Result};
use kairo_core::Config;
use toml_edit::{value, DocumentMut, Item};

pub struct SettingsDocument {
    document: DocumentMut,
    original: Option<Vec<u8>>,
    pub config: Config,
}

impl SettingsDocument {
    pub fn load(files: &ProjectFiles) -> Result<Self> {
        let original = if files.filesystem().exists("kairo.toml")? {
            Some(files.read("kairo.toml")?)
        } else {
            None
        };
        let source = std::str::from_utf8(original.as_deref().unwrap_or_default())?;
        Ok(Self {
            document: source.parse().context("cannot parse kairo.toml")?,
            config: Config::parse(source)?,
            original,
        })
    }

    pub fn source(&self) -> Result<String> {
        self.config.validate()?;
        let mut doc = self.document.clone();
        let config = &self.config;
        set_value(&mut doc["game"]["title"], value(config.game.title.clone()));
        set_value(
            &mut doc["game"]["width"],
            value(i64::from(config.game.width)),
        );
        set_value(
            &mut doc["game"]["height"],
            value(i64::from(config.game.height)),
        );
        set_value(&mut doc["game"]["vsync"], value(config.game.vsync));
        set_value(&mut doc["game"]["resizable"], value(config.game.resizable));
        set_value(&mut doc["audio"]["enabled"], value(config.audio.enabled));
        set_value(
            &mut doc["development"]["hot_reload"],
            value(config.development.hot_reload),
        );
        set_value(
            &mut doc["physics"]["gravity_x"],
            value(f64::from(config.physics.gravity_x)),
        );
        set_value(
            &mut doc["physics"]["gravity_y"],
            value(f64::from(config.physics.gravity_y)),
        );
        set_value(
            &mut doc["physics"]["pixels_per_meter"],
            value(f64::from(config.physics.pixels_per_meter)),
        );
        set_value(
            &mut doc["save"]["identity"],
            value(config.save.identity.clone()),
        );
        set_value(&mut doc["micro"]["enabled"], value(config.micro.enabled));
        set_value(
            &mut doc["micro"]["width"],
            value(i64::from(config.micro.width)),
        );
        set_value(
            &mut doc["micro"]["height"],
            value(i64::from(config.micro.height)),
        );
        set_value(
            &mut doc["micro"]["integer_scaling"],
            value(config.micro.integer_scaling),
        );
        set_value(
            &mut doc["micro"]["pixel_snap"],
            value(config.micro.pixel_snap),
        );
        set_value(
            &mut doc["micro"]["palette"],
            value(config.micro.palette.clone()),
        );
        set_value(&mut doc["replay"]["enabled"], value(config.replay.enabled));
        set_value(&mut doc["replay"]["seconds"], value(config.replay.seconds));
        set_value(
            &mut doc["replay"]["frequency"],
            value(i64::from(config.replay.frequency)),
        );
        set_value(
            &mut doc["replay"]["memory_mib"],
            value(config.replay.memory_mib as i64),
        );
        let source = doc.to_string();
        Config::parse(&source)?;
        Ok(source)
    }

    pub fn save(&mut self, files: &ProjectFiles) -> Result<()> {
        let current = if files.filesystem().exists("kairo.toml")? {
            Some(files.read("kairo.toml")?)
        } else {
            None
        };
        ensure!(
            current == self.original,
            "kairo.toml changed on disk; reopen settings before saving"
        );
        let source = self.source()?;
        files.write("kairo.toml", source.as_bytes())?;
        self.document = source.parse()?;
        self.original = Some(source.into_bytes());
        Ok(())
    }
}

fn set_value(item: &mut Item, mut value: Item) {
    if let (Some(old), Some(new)) = (item.as_value(), value.as_value_mut()) {
        *new.decor_mut() = old.decor().clone();
    }
    *item = value;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editing_known_settings_preserves_unknown_tables_and_comments() {
        let root = tempfile::tempdir().unwrap();
        let files = ProjectFiles::open(root.path()).unwrap();
        files
            .create_file(
                "kairo.toml",
                b"# my game\n[game]\ntitle='Original' # keep this note\n[custom]\nlevel=7\n",
            )
            .unwrap();
        let mut doc = SettingsDocument::load(&files).unwrap();
        doc.config.game.title = "Edited".into();
        doc.save(&files).unwrap();
        let source = files.filesystem().read_text("kairo.toml").unwrap();
        assert!(source.contains("# my game"));
        assert!(source.contains("# keep this note"));
        assert!(source.contains("[custom]"));
        assert!(source.contains("level=7"));
        assert_eq!(Config::parse(&source).unwrap().game.title, "Edited");
    }

    #[test]
    fn settings_refuse_to_overwrite_external_changes() {
        let root = tempfile::tempdir().unwrap();
        let files = ProjectFiles::open(root.path()).unwrap();
        let mut doc = SettingsDocument::load(&files).unwrap();
        files
            .create_file("kairo.toml", b"[game]\ntitle='External'\n")
            .unwrap();
        assert!(doc.save(&files).is_err());
    }
}
