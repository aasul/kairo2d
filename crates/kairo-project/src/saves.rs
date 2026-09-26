//! Keeps per-user game saves outside both the installation and project folders.
use crate::ProjectFiles;
use anyhow::{ensure, Context, Result};
use std::path::Path;

const MAX_SAVE: usize = 4 * 1024 * 1024;

pub struct SaveStore {
    files: ProjectFiles,
}

impl SaveStore {
    pub fn open(identity: &str) -> Result<Self> {
        validate_identity(identity)?;
        let directories = directories::ProjectDirs::from("org", "Kairo2D", identity)
            .context("cannot determine a per-user save directory")?;
        Self::at(directories.data_local_dir())
    }

    pub fn at(root: &Path) -> Result<Self> {
        std::fs::create_dir_all(root).context("cannot create save directory")?;
        Ok(Self {
            files: ProjectFiles::open(root)?,
        })
    }

    pub fn write(&self, name: &str, value: &serde_json::Value) -> Result<()> {
        validate_name(name)?;
        let bytes = serde_json::to_vec_pretty(value)?;
        ensure!(bytes.len() <= MAX_SAVE, "save exceeds 4 MiB");
        self.files.write(name, &bytes)
    }

    pub fn read(&self, name: &str) -> Result<Option<serde_json::Value>> {
        validate_name(name)?;
        if !self.files.filesystem().exists(name)? {
            return Ok(None);
        }
        let bytes = self.files.read(name)?;
        ensure!(bytes.len() <= MAX_SAVE, "save exceeds 4 MiB");
        Ok(Some(
            serde_json::from_slice(&bytes).context("save is not valid JSON")?,
        ))
    }

    pub fn exists(&self, name: &str) -> Result<bool> {
        validate_name(name)?;
        self.files.filesystem().exists(name)
    }

    pub fn remove(&self, name: &str) -> Result<bool> {
        if !self.exists(name)? {
            return Ok(false);
        }
        self.files.delete(Path::new(name))?;
        Ok(true)
    }
}

fn validate_identity(identity: &str) -> Result<()> {
    ensure!(
        !identity.is_empty()
            && identity.len() <= 96
            && identity
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_')),
        "set [save] identity in kairo.toml: 1..=96 ASCII letters, digits, '-' or '_'"
    );
    Ok(())
}

fn validate_name(name: &str) -> Result<()> {
    ensure!(
        name.ends_with(".json")
            && name.len() <= 128
            && !name.starts_with('.')
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.')),
        "save name must be a simple .json filename, without directory components"
    );
    ensure!(!name.contains(".."), "save names cannot contain '..'");
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let numbered_device = stem.len() == 4
        && (stem.starts_with("COM") || stem.starts_with("LPT"))
        && matches!(stem.as_bytes()[3], b'1'..=b'9');
    ensure!(
        !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL") && !numbered_device,
        "save name is a reserved Windows device name"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saves_roundtrip_and_do_not_follow_parent_paths() {
        let root = tempfile::tempdir().unwrap();
        let store = SaveStore::at(root.path()).unwrap();
        let value = serde_json::json!({ "score": 42, "volume": 0.5 });
        store.write("settings.json", &value).unwrap();
        assert_eq!(store.read("settings.json").unwrap(), Some(value));
        for invalid in [
            "../settings.json",
            "C:/settings.json",
            "a\\b.json",
            ".env",
            "x/../y.json",
            "CON.json",
            "nul.backup.json",
            "COM1.json",
            "LPT9.json",
        ] {
            assert!(store.write(invalid, &serde_json::Value::Null).is_err());
        }
        assert!(store.remove("settings.json").unwrap());
        assert_eq!(store.read("settings.json").unwrap(), None);
    }
    #[test]
    fn bad_identity_and_oversized_save_are_rejected() {
        assert!(validate_identity("../game").is_err());
        assert!(validate_identity("").is_err());
        let root = tempfile::tempdir().unwrap();
        let store = SaveStore::at(root.path()).unwrap();
        assert!(store
            .write("big.json", &serde_json::Value::String("x".repeat(MAX_SAVE)))
            .is_err());
    }
}
