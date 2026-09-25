use anyhow::{bail, ensure, Context, Result};
use kairo_core::ProjectFs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

/// Editor mutations reject symlinks, even those pointing back into the project.
/// This avoids accidentally editing a different file through an innocent-looking node.
#[derive(Clone, Debug)]
pub struct ProjectFiles {
    fs: ProjectFs,
}

#[derive(Clone, Debug)]
pub struct ProjectNode {
    pub relative: PathBuf,
    pub directory: bool,
    pub bytes: u64,
}

impl ProjectFiles {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            fs: ProjectFs::new(root)?,
        })
    }

    pub fn filesystem(&self) -> &ProjectFs {
        &self.fs
    }

    pub fn root(&self) -> &Path {
        self.fs.root()
    }

    pub fn resolve(&self, relative: impl AsRef<Path>) -> Result<PathBuf> {
        let relative = relative.as_ref();
        let mut path = self.root().to_owned();
        ensure!(
            !relative.as_os_str().is_empty(),
            "choose a project-relative path"
        );
        for component in relative.components() {
            match component {
                Component::Normal(name) => {
                    let name = name.to_str().context("filenames must be UTF-8")?;
                    ensure!(
                        !name.contains(['\\', ':']),
                        "use portable project-relative filenames"
                    );
                    path.push(name);
                    match path.symlink_metadata() {
                        Ok(metadata) => ensure!(
                            !metadata.file_type().is_symlink(),
                            "symlinks cannot be edited"
                        ),
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                        Err(error) => return Err(error).context("cannot inspect project path"),
                    }
                }
                Component::CurDir => {}
                _ => bail!("path must stay inside the project and cannot contain '..'"),
            }
        }
        ensure!(path != self.root(), "the project root cannot be modified");
        Ok(path)
    }

    pub fn read(&self, relative: impl AsRef<Path>) -> Result<Vec<u8>> {
        self.resolve(&relative)?;
        self.fs.read(relative)
    }

    pub fn write(&self, relative: impl AsRef<Path>, bytes: &[u8]) -> Result<()> {
        let path = self.resolve(relative)?;
        ensure!(
            path.parent().is_some_and(Path::is_dir),
            "parent folder does not exist"
        );
        atomic_write(&path, bytes)
    }

    pub fn create_file(&self, relative: impl AsRef<Path>, bytes: &[u8]) -> Result<()> {
        let path = self.resolve(relative)?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .with_context(|| format!("cannot create {}", path.display()))?;
        if let Err(error) = file.write_all(bytes).and_then(|_| file.sync_all()) {
            drop(file);
            let _ = std::fs::remove_file(&path);
            return Err(error).context("cannot write new file");
        }
        Ok(())
    }

    pub fn create_directory(&self, relative: impl AsRef<Path>) -> Result<()> {
        std::fs::create_dir(self.resolve(relative)?).context("cannot create folder")
    }

    pub fn rename(&self, from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<()> {
        let from = self.resolve(from)?;
        let to = self.resolve(to)?;
        ensure!(!to.try_exists()?, "destination already exists");
        ensure!(from.try_exists()?, "source no longer exists");
        std::fs::rename(from, to).context("cannot rename project file")
    }

    pub fn duplicate(&self, from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<()> {
        let bytes = self.read(from)?;
        self.create_file(to, &bytes)
    }

    pub fn delete(&self, relative: impl AsRef<Path>) -> Result<()> {
        let path = self.resolve(relative)?;
        if path.is_dir() {
            // Refuse a tree containing symlinks instead of following them during deletion.
            validate_tree(&path, 0)?;
            std::fs::remove_dir_all(path).context("cannot delete folder")
        } else {
            std::fs::remove_file(path).context("cannot delete file")
        }
    }

    pub fn list(&self) -> Result<Vec<ProjectNode>> {
        let mut nodes = Vec::new();
        list_directory(self.root(), self.root(), 0, &mut nodes)?;
        Ok(nodes)
    }
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().context("file has no parent directory")?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".kairo-write-")
        .suffix(".tmp")
        .tempfile_in(parent)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(path)
        .map_err(|error| error.error)
        .context("cannot replace file")?;
    Ok(())
}

pub(crate) fn excluded(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".github"
            | ".kairo"
            | "target"
            | "dist"
            | ".DS_Store"
            | "Thumbs.db"
            | ".idea"
            | ".vscode"
            | ".venv"
            | "__pycache__"
            | ".env"
    ) || name.starts_with(".env.")
        || name.starts_with(".kairo-write-")
        || name.starts_with(".kairo-export-")
        || name.ends_with(".token")
        || name.ends_with(".kairo-token")
        || name.ends_with(".link-token")
        || name == "session-token.txt"
}

fn list_directory(
    root: &Path,
    directory: &Path,
    depth: usize,
    nodes: &mut Vec<ProjectNode>,
) -> Result<()> {
    ensure!(depth <= 64, "project nesting exceeds 64 folders");
    let mut entries = std::fs::read_dir(directory)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let name = entry.file_name();
        if excluded(&name.to_string_lossy()) {
            continue;
        }
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            continue;
        }
        if !kind.is_dir() && !kind.is_file() {
            continue;
        }
        ensure!(
            nodes.len() < 20_000,
            "project browser limit is 20000 files and folders"
        );
        let path = entry.path();
        nodes.push(ProjectNode {
            relative: path.strip_prefix(root)?.to_owned(),
            directory: kind.is_dir(),
            bytes: if kind.is_file() {
                entry.metadata()?.len()
            } else {
                0
            },
        });
        if kind.is_dir() {
            list_directory(root, &path, depth + 1, nodes)?;
        }
    }
    Ok(())
}

fn validate_tree(directory: &Path, depth: usize) -> Result<()> {
    ensure!(depth <= 64, "folder nesting exceeds 64 levels");
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        ensure!(
            !kind.is_symlink(),
            "delete symlinks outside Kairo before deleting this folder"
        );
        if kind.is_dir() {
            validate_tree(&entry.path(), depth + 1)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutations_reject_escapes_and_the_root() {
        let root = tempfile::tempdir().unwrap();
        let files = ProjectFiles::open(root.path()).unwrap();
        for path in ["../outside", ".", "", "C:\\secret", "folder/../../outside"] {
            assert!(files.resolve(path).is_err(), "{path}");
        }
        assert!(files.resolve(root.path()).is_err());
    }

    #[test]
    fn file_operations_do_not_overwrite_existing_destinations() {
        let root = tempfile::tempdir().unwrap();
        let files = ProjectFiles::open(root.path()).unwrap();
        files.create_directory("scripts").unwrap();
        files.create_file("scripts/a.lua", b"return 1").unwrap();
        assert!(files.create_file("scripts/a.lua", b"bad").is_err());
        files.duplicate("scripts/a.lua", "scripts/b.lua").unwrap();
        assert!(files.rename("scripts/a.lua", "scripts/b.lua").is_err());
        files.write("scripts/b.lua", b"return 2").unwrap();
        assert_eq!(files.read("scripts/a.lua").unwrap(), b"return 1");
        files.rename("scripts/b.lua", "scripts/c.lua").unwrap();
        files.delete("scripts/c.lua").unwrap();
        assert_eq!(files.list().unwrap().len(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn mutations_cannot_follow_symlinks() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret"), b"original").unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("link")).unwrap();
        let files = ProjectFiles::open(root.path()).unwrap();
        assert!(files.write("link/secret", b"modified").is_err());
        assert!(files.delete("link").is_err());
        assert!(files.list().unwrap().is_empty());
        assert_eq!(
            std::fs::read(outside.path().join("secret")).unwrap(),
            b"original"
        );
    }
}
