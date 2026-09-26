# Project profiles

Project profiles configure runtime policy. They do not compile Rust, switch Cargo optimization, compress assets, or cross-compile a game.

```sh
kairo run . --profile development
kairo run . --profile debug
kairo build . --profile release --output dist/release
```

The main editor toolbar selects Development, Debug, Release or Micro; both Run and Build pass the selection to the runtime. CLI Run defaults development; CLI Build defaults release. **Editor Build uses the selected profile**, so choose Release explicitly before shipping.

Development uses the normal project configuration. Debug additionally shows the profiler. Micro enables the existing virtual-canvas profile without changing physical window size. Release defaults to no hot reload, no runtime tools, no Link permission, no profiler, no automatic Replay recording, and warning-level logging. These policies do not remove the game-facing replay module: a game's own explicit `replay.enable(true)` remains possible.

Overrides in `kairo.toml`:

```toml
[profile.release]
hot_reload = false
tools = false
profiler = false
link = false
replay = false
log_level = "warn"
vsync = true
width = 1280
height = 720
```

Supported override keys: `hot_reload`, `tools`, `profiler`, `link`, `replay`, `micro`, `width`, `height`, `vsync`, `log_level`. Logging levels: off/error/warn/info/debug/trace. `RUST_LOG` overrides default logger policy. Link=true means permission to use an explicitly requested session, not automatic network startup. Tools=false blocks typed external runtime controls and the debug overlay/debugui callback.

Exports store the chosen profile in `kairo-package.toml`. No-argument launch of the packaged game applies it. Older markers without a profile default to release. Calling the runtime with an explicit `run` command uses that command's selected profile instead.

`cargo build --release` independently produces an optimized native runtime/editor. `kairo build` copies the supplied runtime and project data; it does not change that executable's optimization. There is no UI control pretending otherwise.
