# Editor and source packaging

## Source

```sh
python scripts/source_audit.py
python scripts/generate_api_docs.py --check
python scripts/package.py --output ../kairo2d-3.4.0.zip
```

The source archive has one `kairo2d-3.4.0/` root, required-entry/CRC checks and a SHA-256
sidecar. It excludes target, dist, .git, caches, local credential patterns and generated
license reports. The actual delivered ZIP is independently extracted and byte-checked.
Archive integrity is not compilation evidence.

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

The native artifact workflow resolves a shared lockfile, builds on Windows/Linux/macOS,
executes headless examples and packages only on success. It is configured but was not
run during authoring. Run/manual GUI acceptance, signing and dependency review remain
release tasks even if a future CI build succeeds.

The absence of Cargo in the authoring environment means **no native artifact is included
with the 3.4 source delivery**. There is no fake Kairo.exe or fabricated Cargo.lock.
