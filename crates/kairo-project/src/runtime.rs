//! Finds the game runtime for the editor and packaging tools.
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct RuntimeProbe {
    pub path: PathBuf,
    pub outcome: String,
}

#[derive(Clone, Debug, Default)]
pub struct RuntimeDiscovery {
    pub executable: Option<PathBuf>,
    pub probes: Vec<RuntimeProbe>,
}

impl RuntimeDiscovery {
    pub fn diagnostic(&self) -> String {
        let mut text = String::from("Kairo runtime discovery\n");
        for probe in &self.probes {
            text.push_str(&format!("  {}: {}\n", probe.path.display(), probe.outcome));
        }
        if self.executable.is_none() {
            text.push_str("Build both binaries with cargo build --workspace, or select the engine runtime in Preferences.\nFor a Windows release use Kairo.exe + bin/kairo.exe. Do not select the editor itself.");
        }
        text
    }

    pub fn require(self) -> Result<PathBuf> {
        match &self.executable {
            Some(path) => Ok(path.clone()),
            None => bail!("{}", self.diagnostic()),
        }
    }
}

pub fn discover_runtime(
    configured: Option<&Path>,
    editor: &Path,
    workspace: Option<&Path>,
    target_dir: Option<&Path>,
) -> RuntimeDiscovery {
    let mut discovery = RuntimeDiscovery::default();
    let editor = editor.canonicalize().unwrap_or_else(|_| editor.to_owned());
    for candidate in runtime_candidates(configured, &editor, workspace, target_dir) {
        let outcome = match candidate.canonicalize() {
            Ok(path) if path == editor => "is the editor, not the runtime".into(),
            Ok(path) if !path.is_file() => "not a regular file".into(),
            Ok(path) => match crate::package::validate_native_executable(&path) {
                Ok(()) => {
                    discovery.executable = Some(path);
                    "found native runtime candidate".into()
                }
                Err(error) => format!("{error:#}"),
            },
            Err(error) => error.to_string(),
        };
        discovery.probes.push(RuntimeProbe {
            path: candidate,
            outcome,
        });
        if discovery.executable.is_some() {
            break;
        }
    }
    discovery
}

pub fn runtime_candidates(
    configured: Option<&Path>,
    editor: &Path,
    workspace: Option<&Path>,
    target_dir: Option<&Path>,
) -> Vec<PathBuf> {
    let name = if cfg!(windows) { "kairo.exe" } else { "kairo" };
    let mut paths = Vec::new();
    if let Some(path) = configured {
        paths.push(path.to_owned());
    }
    if let Some(parent) = editor.parent() {
        paths.push(parent.join("bin").join(name));
        paths.push(parent.join(name));
    }
    if let Some(target) = target_dir {
        let target = if target.is_absolute() {
            target.to_owned()
        } else {
            workspace.unwrap_or(Path::new(".")).join(target)
        };
        paths.push(target.join("debug").join(name));
        paths.push(target.join("release").join(name));
    }
    if let Some(root) = workspace {
        paths.push(root.join("target/debug").join(name));
        paths.push(root.join("target/release").join(name));
    }
    let mut unique = Vec::new();
    for path in paths {
        if !unique.contains(&path) {
            unique.push(path);
        }
    }
    unique
}

pub fn current_runtime(configured: Option<&Path>) -> Result<PathBuf> {
    let editor = std::env::current_exe().context("cannot locate the editor executable")?;
    let cwd = std::env::current_dir().ok();
    let workspace = cwd
        .as_deref()
        .and_then(find_workspace)
        .or_else(|| editor.parent().and_then(find_workspace));
    let target = std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from);
    discover_runtime(configured, &editor, workspace.as_deref(), target.as_deref()).require()
}

fn find_workspace(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .take(8)
        .find(|directory| {
            std::fs::read_to_string(directory.join("Cargo.toml"))
                .ok()
                .is_some_and(|source| source.contains("[workspace]"))
        })
        .map(Path::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_and_explicit_paths_precede_development_fallbacks() {
        let paths = runtime_candidates(
            Some(Path::new("chosen")),
            Path::new("release/Kairo.exe"),
            Some(Path::new("source")),
            Some(Path::new("custom")),
        );
        assert_eq!(paths[0], Path::new("chosen"));
        assert_eq!(paths[1].parent().unwrap(), Path::new("release/bin"));
        assert!(paths
            .iter()
            .any(|path| path.starts_with("source/custom/debug")));
    }

    #[test]
    fn diagnostics_include_every_failed_probe_and_reject_the_editor() {
        let root = tempfile::tempdir().unwrap();
        let editor = std::env::current_exe().unwrap();
        let result = discover_runtime(Some(&editor), &editor, Some(root.path()), None);
        assert!(result.probes[0].outcome.contains("is the editor"));
        assert!(result.diagnostic().contains(&editor.display().to_string()));
    }

    #[test]
    fn portable_release_finds_bin_runtime() {
        let root = tempfile::tempdir().unwrap();
        let bin = root.path().join("bin");
        std::fs::create_dir(&bin).unwrap();
        let name = if cfg!(windows) { "kairo.exe" } else { "kairo" };
        std::fs::copy(std::env::current_exe().unwrap(), bin.join(name)).unwrap();
        let result = discover_runtime(None, &root.path().join("Kairo.exe"), None, None);
        assert_eq!(
            result.executable.unwrap(),
            bin.join(name).canonicalize().unwrap()
        );
    }
}
