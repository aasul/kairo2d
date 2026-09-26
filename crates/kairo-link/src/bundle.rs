use anyhow::{ensure, Context, Result};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub const MAX_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_FILES: usize = 512;
pub(crate) const MAX_FILE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone)]
pub struct SourceFile {
    pub path: String,
    pub bytes: Vec<u8>,
}
#[derive(Clone)]
pub struct Bundle {
    pub revision: String,
    pub files: Vec<SourceFile>,
}

pub(crate) fn validate_path(path: &str) -> Result<()> {
    ensure!(
        !path.is_empty() && path.len() <= 240 && path.is_ascii(),
        "Link paths must be portable ASCII, 1..=240 bytes"
    );
    ensure!(
        !path.contains(['\\', ':', '\0']) && path.bytes().all(|b| b >= 32 && b != 127),
        "invalid Link path characters"
    );
    let components: Vec<_> = path.split('/').collect();
    ensure!(components.len() <= 32, "Link path is nested too deeply");
    for part in components {
        ensure!(
            !part.is_empty() && !part.starts_with('.') && !part.ends_with(['.', ' ']),
            "unsafe Link path component"
        );
        ensure!(
            !part.contains(['<', '>', '"', '|', '?', '*']),
            "nonportable Link path"
        );
        let stem = part
            .split('.')
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        ensure!(
            !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CLOCK$")
                && !(stem.len() == 4
                    && (stem.starts_with("COM") || stem.starts_with("LPT"))
                    && matches!(stem.as_bytes()[3], b'1'..=b'9')),
            "reserved Windows filename"
        );
    }
    let extension = Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    ensure!(
        matches!(
            extension.as_str(),
            "lua"
                | "toml"
                | "json"
                | "tmj"
                | "tsj"
                | "png"
                | "jpg"
                | "jpeg"
                | "wav"
                | "ogg"
                | "ttf"
                | "otf"
        ),
        "unsupported Link file type"
    );
    Ok(())
}

fn scan(
    root: &Path,
    directory: &Path,
    paths: &mut Vec<PathBuf>,
    depth: usize,
    visited: &mut usize,
) -> Result<()> {
    ensure!(depth <= 32, "Link project nesting limit exceeded");
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        *visited += 1;
        ensure!(
            *visited <= 8192,
            "Link project directory scan limit exceeded"
        );
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || matches!(name.as_str(), "target" | "dist" | "node_modules") {
            continue;
        }
        let path = entry.path();
        let kind = entry.file_type()?;
        ensure!(
            !kind.is_symlink(),
            "Link refuses symlinks: {}",
            path.display()
        );
        if kind.is_dir() {
            scan(root, &path, paths, depth + 1, visited)?;
        } else if kind.is_file() {
            let relative = path
                .strip_prefix(root)?
                .to_str()
                .context("Link filenames must be UTF-8")?
                .replace('\\', "/");
            let extension = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !matches!(
                extension.as_str(),
                "lua"
                    | "toml"
                    | "json"
                    | "tmj"
                    | "tsj"
                    | "png"
                    | "jpg"
                    | "jpeg"
                    | "wav"
                    | "ogg"
                    | "ttf"
                    | "otf"
            ) {
                continue;
            }
            validate_path(&relative)?;
            paths.push(path);
            ensure!(
                paths.len() <= MAX_FILES,
                "Link supports at most 512 project files"
            );
        } else {
            anyhow::bail!("Link refuses special files");
        }
    }
    Ok(())
}

fn paths(root: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    scan(root, root, &mut paths, 0, &mut 0)?;
    paths.sort();
    Ok(paths)
}

pub(crate) fn fingerprint(root: &Path) -> Result<Vec<(PathBuf, u64, u128)>> {
    paths(root)?
        .into_iter()
        .map(|path| {
            let metadata = path.metadata()?;
            let modified = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_nanos();
            Ok((path, metadata.len(), modified))
        })
        .collect()
}

impl Bundle {
    pub fn capture(root: &Path) -> Result<Self> {
        let mut files = Vec::new();
        let mut total = 0_usize;
        for path in paths(root)? {
            let metadata = path.metadata()?;
            ensure!(
                metadata.len() <= MAX_FILE_BYTES as u64,
                "Link file exceeds 16 MiB"
            );
            let mut bytes = Vec::new();
            std::fs::File::open(&path)?
                .take(MAX_FILE_BYTES as u64 + 1)
                .read_to_end(&mut bytes)?;
            ensure!(
                bytes.len() <= MAX_FILE_BYTES,
                "Link file grew beyond 16 MiB while reading"
            );
            total = total
                .checked_add(bytes.len())
                .context("Link size overflow")?;
            ensure!(total <= MAX_BYTES, "Link project exceeds 32 MiB");
            let relative = path
                .strip_prefix(root)?
                .to_str()
                .context("non-UTF-8 Link path")?
                .replace('\\', "/");
            files.push(SourceFile {
                path: relative,
                bytes,
            });
        }
        Self::from_files(files)
    }

    pub fn from_files(mut files: Vec<SourceFile>) -> Result<Self> {
        ensure!(
            !files.is_empty() && files.len() <= MAX_FILES,
            "invalid Link file count"
        );
        files.sort_by(|a, b| a.path.cmp(&b.path));
        let mut seen = BTreeSet::new();
        let mut total = 0_usize;
        let mut digest = Sha256::new();
        for file in &files {
            validate_path(&file.path)?;
            ensure!(
                seen.insert(file.path.to_ascii_lowercase()),
                "duplicate/case-colliding Link path"
            );
            ensure!(
                file.bytes.len() <= MAX_FILE_BYTES,
                "Link file exceeds 16 MiB"
            );
            total = total
                .checked_add(file.bytes.len())
                .context("Link size overflow")?;
            ensure!(total <= MAX_BYTES, "Link project exceeds 32 MiB");
            digest.update((file.path.len() as u64).to_le_bytes());
            digest.update(file.path.as_bytes());
            digest.update((file.bytes.len() as u64).to_le_bytes());
            digest.update(&file.bytes);
        }
        ensure!(
            files.iter().any(|f| f.path == "main.lua"),
            "Link project needs main.lua"
        );
        Ok(Self {
            revision: format!("{:x}", digest.finalize()),
            files,
        })
    }

    /// Every revision has its own fresh directory. The live project is never overwritten.
    pub fn stage(&self) -> Result<tempfile::TempDir> {
        let staged = tempfile::Builder::new().prefix("kairo-link-").tempdir()?;
        // Revalidate even bundles constructed by a Rust caller rather than the wire decoder.
        let checked = Self::from_files(self.files.clone())?;
        ensure!(
            checked.revision == self.revision,
            "Link bundle digest mismatch"
        );
        for file in &checked.files {
            let path = staged.path().join(&file.path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, &file.bytes)?;
        }
        Ok(staged)
    }

    pub fn bytes(&self) -> usize {
        self.files.iter().map(|file| file.bytes.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_cross_platform_escape_and_reserved_names() {
        for path in [
            "../a.lua",
            "/main.lua",
            "C:/main.lua",
            "a\\b.lua",
            "a/./x.lua",
            "a/CON.lua",
            "a/NUL.txt",
            "a/b.lua:stream",
            "a./b.lua",
            "script.exe",
            "a/*/b.lua",
        ] {
            assert!(validate_path(path).is_err(), "{path}");
        }
        assert!(validate_path("scripts/player.lua").is_ok());
    }
    #[test]
    fn staging_is_hashed_and_isolated() {
        let bundle = Bundle::from_files(vec![SourceFile {
            path: "main.lua".into(),
            bytes: b"game={}".to_vec(),
        }])
        .unwrap();
        let staged = bundle.stage().unwrap();
        assert_eq!(
            std::fs::read(staged.path().join("main.lua")).unwrap(),
            b"game={}"
        );
        let mut corrupted = bundle.clone();
        corrupted.files[0].bytes.push(b'!');
        assert!(corrupted.stage().is_err());
    }
    #[test]
    fn rejects_case_collisions_and_missing_entrypoint() {
        let file = |path: &str| SourceFile {
            path: path.into(),
            bytes: Vec::new(),
        };
        assert!(Bundle::from_files(vec![file("main.lua"), file("Main.lua")]).is_err());
        assert!(Bundle::from_files(vec![file("other.lua")]).is_err());
    }
}
