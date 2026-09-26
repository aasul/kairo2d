use crate::files::excluded;
use anyhow::{ensure, Context, Result};
use kairo_core::{Config, ProjectFs};
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct PackageReport {
    pub directory: PathBuf,
    pub executable: PathBuf,
    pub files: usize,
    pub bytes: u64,
}

#[derive(Deserialize, Serialize)]
struct PackageMarker {
    format: u32,
    project: String,
    #[serde(default = "release_profile")]
    profile: kairo_core::profile::BuildProfile,
}

fn release_profile() -> kairo_core::profile::BuildProfile {
    kairo_core::profile::BuildProfile::Release
}

/// Copy an already-built host runtime. This is packaging, not cross-compilation.
pub fn build_package(project: &ProjectFs, runtime: &Path, output: &Path) -> Result<PackageReport> {
    build_package_with_profile(project, runtime, output, release_profile())
}

pub fn build_package_with_profile(
    project: &ProjectFs,
    runtime: &Path,
    output: &Path,
    profile: kairo_core::profile::BuildProfile,
) -> Result<PackageReport> {
    let config = Config::load_profile(project, profile)?;
    project.resolve("main.lua")?;
    ensure!(runtime.is_file(), "runtime executable is missing");
    validate_native_executable(runtime)?;
    ensure!(
        !output.try_exists()?,
        "output already exists; choose a new directory"
    );
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent).context("cannot create export parent folder")?;
    let parent = parent.canonicalize()?;
    let output = parent.join(output.file_name().context("output must name a directory")?);
    ensure!(output != project.root(), "cannot replace the project root");
    if parent.starts_with(project.root()) {
        ensure!(
            parent.starts_with(project.root().join("dist")),
            "exports inside a project must go under dist/; choose an external directory otherwise"
        );
    }
    let stage = tempfile::Builder::new()
        .prefix(".kairo-export-")
        .tempdir_in(&parent)?;
    let game = stage.path().join("game");
    std::fs::create_dir(&game)?;
    let mut report = PackageReport {
        directory: output.clone(),
        executable: PathBuf::new(),
        files: 0,
        bytes: 0,
    };
    copy_tree(project.root(), &game, 0, &mut report)?;
    // Exported games never watch their installation directory unless explicitly run with `kairo run`.
    let name = executable_name(&config.game.title);
    let filename = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name
    };
    std::fs::copy(runtime, stage.path().join(&filename))
        .context("cannot copy runtime executable")?;
    let marker = PackageMarker {
        format: 1,
        project: "game".into(),
        profile,
    };
    std::fs::write(
        stage.path().join("kairo-package.toml"),
        toml::to_string(&marker)?,
    )?;
    std::fs::write(
        stage.path().join("KAIRO-LICENSE.txt"),
        include_str!("../../../LICENSE"),
    )?;
    std::fs::write(
        stage.path().join("THIRD_PARTY.md"),
        include_str!("../../../THIRD_PARTY.md"),
    )?;
    // Keep dependent license files alongside runtime distributions when present.
    if let Some(runtime_dir) = runtime.parent() {
        for candidate in [
            runtime_dir.join("dependency-licenses.json"),
            runtime_dir.join("../dependency-licenses.json"),
        ] {
            if candidate.is_file() {
                std::fs::copy(candidate, stage.path().join("dependency-licenses.json"))?;
                break;
            }
        }
    }
    std::fs::write(stage.path().join("README.txt"), format!(
        "{}\n\nLaunch {} from this directory. Keep game/ and kairo-package.toml beside it.\nThis is a native host-platform package, not a cross-platform or single-file build.\nProject assets retain their original license; add your game's license before distribution.\n",
        config.game.title, filename
    ))?;
    std::fs::rename(stage.path(), &output).context("cannot publish export directory")?;
    report.executable = output.join(filename);
    Ok(report)
}

/// Locate a packaged game without depending on the process working directory.
pub fn packaged_project(executable: &Path) -> Result<Option<PathBuf>> {
    let parent = executable.parent().context("executable has no directory")?;
    let marker_path = parent.join("kairo-package.toml");
    if !marker_path.try_exists()? {
        return Ok(None);
    }
    let marker: PackageMarker = toml::from_str(&std::fs::read_to_string(marker_path)?)?;
    ensure!(
        marker.format == 1 && marker.project == "game",
        "unsupported game package marker"
    );
    let parent = parent.canonicalize()?;
    let game = parent
        .join("game")
        .canonicalize()
        .context("packaged game directory is missing")?;
    ensure!(
        game.starts_with(&parent),
        "packaged game must stay beside the executable"
    );
    Ok(Some(game))
}

pub fn packaged_profile(executable: &Path) -> Result<kairo_core::profile::BuildProfile> {
    let parent = executable.parent().context("executable has no directory")?;
    let marker: PackageMarker =
        toml::from_str(&std::fs::read_to_string(parent.join("kairo-package.toml"))?)?;
    ensure!(
        marker.format == 1 && marker.project == "game",
        "unsupported game package marker"
    );
    Ok(marker.profile)
}

fn executable_name(title: &str) -> String {
    let name: String = title
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(64)
        .collect();
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if name.is_empty() || reserved.contains(&name.to_ascii_uppercase().as_str()) {
        "MyGame".into()
    } else {
        name
    }
}

pub(crate) fn validate_native_executable(path: &Path) -> Result<()> {
    let mut file = std::fs::File::open(path)?;
    let mut header = [0; 4];
    file.read_exact(&mut header)
        .context("runtime file is too small")?;
    let native = if cfg!(windows) {
        header.starts_with(b"MZ")
    } else if cfg!(target_os = "macos") {
        matches!(
            header,
            [0xcf, 0xfa, 0xed, 0xfe]
                | [0xfe, 0xed, 0xfa, 0xcf]
                | [0xca, 0xfe, 0xba, 0xbe]
                | [0xca, 0xfe, 0xba, 0xbf]
        )
    } else {
        header == *b"\x7fELF"
    };
    ensure!(
        native,
        "runtime does not have a native executable header for this host"
    );
    if cfg!(windows) {
        file.seek(SeekFrom::Start(0x3c))?;
        let mut offset = [0; 4];
        file.read_exact(&mut offset)
            .context("runtime has a truncated DOS header")?;
        let offset = u64::from(u32::from_le_bytes(offset));
        ensure!(
            offset >= 64 && offset <= file.metadata()?.len().saturating_sub(4),
            "invalid runtime PE offset"
        );
        file.seek(SeekFrom::Start(offset))?;
        file.read_exact(&mut header)?;
        ensure!(
            &header == b"PE\0\0",
            "runtime is missing the Windows PE signature"
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        ensure!(
            file.metadata()?.permissions().mode() & 0o111 != 0,
            "runtime file has no executable permission"
        );
    }
    Ok(())
}

fn copy_tree(
    source: &Path,
    destination: &Path,
    depth: usize,
    report: &mut PackageReport,
) -> Result<()> {
    ensure!(depth <= 64, "project nesting exceeds 64 levels");
    let mut entries = std::fs::read_dir(source)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let name = entry.file_name();
        if excluded(&name.to_string_lossy()) {
            continue;
        }
        let kind = entry.file_type()?;
        ensure!(
            !kind.is_symlink(),
            "exports do not follow symlinks: {}",
            entry.path().display()
        );
        let to = destination.join(&name);
        if kind.is_dir() {
            std::fs::create_dir(&to)?;
            copy_tree(&entry.path(), &to, depth + 1, report)?;
        } else {
            ensure!(kind.is_file(), "project contains a non-regular file");
            report.files += 1;
            report.bytes = report
                .bytes
                .checked_add(entry.metadata()?.len())
                .context("project size overflow")?;
            ensure!(
                report.files <= 20_000 && report.bytes <= 2 * 1024 * 1024 * 1024,
                "export exceeds 20000 files or 2 GiB"
            );
            std::fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{create_project, Template};

    #[test]
    fn export_contains_runtime_assets_and_a_relocatable_marker() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        create_project(&source, "Test Game", Template::HelloWorld).unwrap();
        std::fs::create_dir(source.join("target")).unwrap();
        std::fs::write(source.join("target/junk"), b"excluded").unwrap();
        std::fs::write(source.join(".env"), b"do not distribute").unwrap();
        let fs = ProjectFs::new(&source).unwrap();
        let runtime = std::env::current_exe().unwrap();
        let output = root.path().join("release");
        let report = build_package(&fs, &runtime, &output).unwrap();
        assert!(report.executable.is_file());
        assert!(output.join("game/main.lua").is_file());
        assert!(!output.join("game/target").exists());
        assert!(!output.join("game/.env").exists());
        assert_eq!(
            packaged_project(&report.executable).unwrap().unwrap(),
            output.join("game").canonicalize().unwrap()
        );
        assert!(build_package(&fs, &runtime, &output).is_err());
    }

    #[test]
    fn truncated_or_text_runtime_is_rejected() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("not-a-runtime");
        std::fs::write(&path, b"MZ").unwrap();
        assert!(validate_native_executable(&path).is_err());
        std::fs::write(&path, b"this is source, not a binary").unwrap();
        assert!(validate_native_executable(&path).is_err());
    }

    #[test]
    fn unsafe_or_empty_titles_get_portable_executable_names() {
        assert_eq!(executable_name("../My Game!"), "MyGame");
        assert_eq!(executable_name("CON"), "MyGame");
        assert_eq!(executable_name("Breakout"), "Breakout");
    }

    #[test]
    fn rejects_arbitrary_in_project_export_directories() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("game");
        create_project(&source, "Test", Template::Empty).unwrap();
        let fs = ProjectFs::new(&source).unwrap();
        assert!(build_package(
            &fs,
            &std::env::current_exe().unwrap(),
            &source.join("release")
        )
        .is_err());
    }
}
