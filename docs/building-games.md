# Building standalone games

Build a native release runtime first. A source ZIP alone cannot run a game:

```sh
cargo build --release --workspace -j 1
./target/release/kairo build mygame --profile release --output exports/MyGame
```

Windows equivalent: `target\release\kairo.exe build mygame --profile release --output exports\MyGame`.
The editor Export action uses the selected runtime and the same shared package
service. A debug runtime produces a debug game package; select the release runtime
before shipping. The editor profile selector also controls Export; use Release.
The profile disables runtime tooling defaults, but does not optimize a debug executable.

The resulting directory contains:

```text
MyGame.exe                 # or a native non-Windows binary
kairo-package.toml
game/
  kairo.toml
  main.lua
  assets/
KAIRO-LICENSE.txt
README.txt
dependency-licenses.json   # when available beside the source runtime
```

Names derive from the title after portable-name filtering. The package marker points
to `game/` and records the selected runtime profile; launching with no arguments
uses that profile and locates the project relative to the executable, not
the current working directory. Preserve the marker and entire directory.

Export validates config/Lua syntax; it does not run every gameplay branch. It refuses
existing output folders, unsafe symlinks/special files, more than 20,000 copied files
or over 2 GiB of game data. Hidden development/cache/credential patterns are excluded;
this is not a complete secret scanner. Review the output yourself.

The package copies the **host** runtime, not a newly cross-compiled runtime. Players
need no Rust installation, but compatible system graphics/audio/window libraries
are still required. Windows release builds target MSVC and request a static C runtime.
Linux packages are native directories, not universally portable AppImages. macOS
packages are unsigned. No single-file embedding, DLL dependency crawler, installer,
notarization or code signing is provided.

Before public distribution, generate and review dependency notices for the exact
resolved Cargo.lock:

```sh
cargo install cargo-bundle-licenses --version 4.2.0 --locked
cargo bundle-licenses --format json --output dependency-licenses.json
```

Copy the report beside the runtime used to export, or use a desktop release package
that includes it. Missing notices do not stop ordinary development exports, but they
must be resolved before redistribution. Add licenses for your own fonts, images,
audio and game source.

Acceptance test on a clean target machine/account with no Rust installed: launch
from a directory unrelated to the package, exercise save data, controller/audio and
restart, and check missing-file/error handling. No packaged binary acceptance was
performed during authoring of this snapshot.

## Export regression checks

After building the real runtime:

```sh
python scripts/package_smoke.py --binary target/release/kairo
```

On Windows append `.exe`. It creates a temporary project, verifies non-overwrite,
exports it, checks excluded files and notices, relocates the distribution, removes
the source, then runs the real exported runtime headlessly. Add `--windowed` on a
graphical machine to test no-argument package-marker discovery with a self-closing
three-frame game. This script was written but could not run without a built runtime
in the authoring environment; it is part of the configured CI/release gate.
