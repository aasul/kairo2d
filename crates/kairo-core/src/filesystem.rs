use anyhow::{bail, ensure, Context, Result};
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

const MAX_READ_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct ProjectFs {
    root: PathBuf,
}

impl ProjectFs {
    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root = root
            .as_ref()
            .canonicalize()
            .context("cannot open game directory")?;
        ensure!(root.is_dir(), "game project must be a directory");
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn candidate(&self, relative: impl AsRef<Path>) -> Result<PathBuf> {
        let path = relative.as_ref();
        for component in path.components() {
            match component {
                Component::Normal(_) | Component::CurDir => {}
                _ => bail!("project paths must be relative and cannot contain '..'"),
            }
        }
        Ok(self.root.join(path))
    }

    pub fn resolve(&self, relative: impl AsRef<Path>) -> Result<PathBuf> {
        let relative = relative.as_ref();
        let path = self
            .candidate(relative)?
            .canonicalize()
            .with_context(|| format!("cannot open {}", relative.display()))?;
        ensure!(
            path.starts_with(&self.root),
            "path escapes the game project"
        );
        Ok(path)
    }

    pub fn exists(&self, relative: impl AsRef<Path>) -> Result<bool> {
        let candidate = self.candidate(relative)?;
        match candidate.canonicalize() {
            Ok(path) => {
                ensure!(
                    path.starts_with(&self.root),
                    "path escapes the game project"
                );
                Ok(true)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error).context("cannot inspect project path"),
        }
    }

    pub fn read(&self, relative: impl AsRef<Path>) -> Result<Vec<u8>> {
        let relative = relative.as_ref();
        let path = self.resolve(relative)?;
        let file =
            File::open(path).with_context(|| format!("cannot read {}", relative.display()))?;
        ensure!(file.metadata()?.is_file(), "path is not a regular file");
        ensure!(
            file.metadata()?.len() <= MAX_READ_BYTES,
            "file exceeds 64 MiB read limit"
        );
        let mut bytes = Vec::new();
        file.take(MAX_READ_BYTES + 1).read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() as u64 <= MAX_READ_BYTES,
            "file exceeds 64 MiB read limit"
        );
        Ok(bytes)
    }

    pub fn read_text(&self, relative: impl AsRef<Path>) -> Result<String> {
        String::from_utf8(self.read(relative)?).context("file is not valid UTF-8")
    }

    pub fn list(&self, relative: impl AsRef<Path>) -> Result<Vec<String>> {
        let path = self.resolve(relative)?;
        ensure!(path.is_dir(), "path is not a directory");
        let mut names = Vec::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let resolved = entry.path().canonicalize()?;
            if resolved.starts_with(&self.root) {
                if let Some(name) = entry.file_name().to_str() {
                    names.push(name.to_owned());
                }
            }
        }
        names.sort();
        Ok(names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_parent_and_absolute_paths() {
        let root = tempfile::tempdir().unwrap();
        let fs = ProjectFs::new(root.path()).unwrap();
        assert!(fs.exists("../outside").is_err());
        assert!(fs.resolve(root.path()).is_err());
        assert!(!fs.exists("missing").unwrap());
    }

    #[test]
    fn reads_relative_paths_and_sorts_directory_entries() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("z.txt"), "hello").unwrap();
        std::fs::write(root.path().join("a.txt"), "world").unwrap();
        let fs = ProjectFs::new(root.path()).unwrap();
        assert_eq!(fs.read_text("./z.txt").unwrap(), "hello");
        assert_eq!(fs.list(".").unwrap(), ["a.txt", "z.txt"]);
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_cannot_escape_the_project() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret"), "not a game asset").unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("link")).unwrap();
        let fs = ProjectFs::new(root.path()).unwrap();
        assert!(fs.read("link/secret").is_err());
        assert!(fs.exists("link/secret").is_err());
    }
}
