# Editor and source packaging

## Source

```sh
python scripts/source_audit.py
python scripts/generate_api_docs.py --check
python scripts/package.py --output ../kairo2d-3.4.0.zip
```

The source archive has one `kairo2d-3.4.0/` root and includes required-file, CRC, and
SHA-256 checks. Build output, Git metadata, caches, and local credential files are
left out. The archive is extracted and checked after it is created. These checks
confirm the contents of the ZIP, not that the Rust code builds.

## Native desktop

First resolve dependencies, format/check/test/Clippy, build both release executables,
and generate a reviewed `dependency-licenses.json`. The script refuses missing or
foreign executable headers rather than renaming a non-Windows binary to .exe.

```sh
python scripts/desktop_release.py --build-dir target/release
```

It packages `Kairo`/`Kairo.exe` and `bin/kairo`/`bin/kairo.exe`, notices, examples and
docs into a host-specific ZIP with a checksum. `--build-dir` supports custom Cargo
target directories. Windows PowerShell automation:

```powershell
.\tools\build-windows.ps1 -LowDisk -Package
```

The script targets x86_64 MSVC by default and formats source before strict validation.
It does not install MSVC/Rust. `-TargetDirectory` relocates build outputs; `-Target`
selects an already-supported installed native Windows target. This is not a general
cross-compilation service. Use the same target directory/toolchain for both binaries.

The desktop release workflow resolves one lockfile, builds on Windows, Linux, and
macOS, runs the headless examples, and packages only after those steps pass. It still
needs a run on each platform, followed by hands-on editor checks, signing, and a
review of the dependencies included in the release.

The 3.4.0 source release does not include a native desktop build. Build the editor and
runtime from this repository for your platform before packaging a game.
