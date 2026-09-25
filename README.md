# Kairo2D 3.4.0

**Lua-first 2D games. Rust underneath. Kairo on the desktop.**

Kairo2D is an MIT-licensed game engine with its own queued 2D renderer and Lua 5.4
API. Kairo is the accompanying Rust desktop editor. Version 3.4 adds practical game
systems and makes local and remote inspection part of the development workflow.

> **Uncompiled development source snapshot, not a verified binary release.**
> This repository extends the latest 3.3 Feature UI source, retaining its visible
> feature controls and earlier fixes. The authoring environment had no Cargo, rustc,
> rustfmt or PowerShell; toolchain installation failed. Rust tests and desktop,
> graphics, audio, controller and network execution have not run here. Compiler,
> formatting, Clippy and runtime defects may remain. No executable, fabricated
> lockfile, successful CI result or simulated runtime screenshot is included.
> See [VALIDATION.md](VALIDATION.md) for the checks actually performed.

## What's new in 3.4

| Area | Implemented source behavior |
| --- | --- |
| Scenes | Named states, queued switch/push/pop/reload, pause overlays, lifecycle callbacks and shared data. |
| Input actions | Keyboard/mouse/gamepad actions, edge states, named axes, dead zones and editable `input.toml`. |
| Live Inspector | Explicitly exposed typed values, read-only defaults, opt-in edits, ranges/choices, stale-edit checks and scene controls. |
| Remote Inspector | The same typed controls over authenticated Link; per-tester telemetry, labels, reload selection and connection status. |
| Animation | Named clip states, predicates, completion edges and transition callbacks. |
| Particles | Rust-side bounded emitters, presets, textured quads and a visual editor using the same simulation. |
| Audio | Master/music/sfx/ui buses, volume/mute/fades, bus stop, playback pitch and pan. |
| Maps and debugging | Tiled properties, tile animation, parallax and rectangle collision data; body/bounds/velocity debug drawing. |
| Replay | Session-local bug bookmarks that pin bounded state beyond rolling-history eviction, with notes and metadata export. |
| Project workflow | Runtime profiles, command palette, background text search, fuzzy quick-open, recent tabs and offline API snippets. |
| Game data | Prefab defaults/overrides, versioned save migrations and language tables with fallback/interpolation. |

These are source implementations, not claims of native validation. Link, Replay,
inspector transport and statically compiled extension APIs remain experimental.
[Known limitations](docs/known-limitations.md) identifies omitted features explicitly.

Existing 3.3 functionality remains: sprite transforms/regions/tint and adjacent
batching; font atlas text; physics; WAV/Ogg playback; gamepads; Lua UI and native
pointer-driven debug UI; Kairo Micro; texture/script reload; code and sprite editing;
shared host-native export; runtime discovery diagnostics.

## Prerequisites and installation

The supplied artifact is source. Install a current stable Rust toolchain with
Cargo, rustfmt and Clippy, a native C/C++ toolchain (vendored Lua is compiled), and
Python 3.11+ for repository checks/packaging. No separate Lua installation is required
for the engine. The optional Lua-only sanity script needs a native Lua 5.4 shared library.

On Windows use the Visual Studio C++ build tools, Windows SDK and Rust's
`x86_64-pc-windows-msvc` toolchain for the configured distribution workflow. On Linux
install the development packages listed in `.github/workflows/ci.yml` (including
ALSA, udev, X11/Wayland and OpenGL support). macOS requires its native developer tools.
GPU/audio drivers and controller hardware are needed for relevant runtime checks.
The bundled scripts do not install prerequisites.

Extract into a **new directory**. Preserve your existing projects and local source
changes; do not copy an old packaged editor over the new build. After resolving
Cargo dependencies, retain the actual generated `Cargo.lock` for reproducible builds.
Do not fabricate one or copy it from an unrelated project.

## Quick start

From the extracted repository root, build both programs:

```sh
cargo fmt --all
cargo build --workspace -j 1
cargo run -p kairo-editor -- examples/top-down
```

Windows helper, including native checks before launch:

```powershell
.\tools\launch-editor.ps1 -LowDisk -Validate
```

The editor title says **3.4 Workflow Tools**. The persistent feature bar exposes
Fantasy Console, Link, Replay, Profiler, Lua UI, Inspector, Input Mappings, Particles
and Mixer. The main toolbar selects a runtime profile. Command palette:
**Ctrl/Cmd+Shift+P**. Run: **F5**. Stop: **Shift+F5**.

The runtime is a separate executable. A packaged Windows editor needs:

```text
Kairo/
  Kairo.exe
  bin/kairo.exe
```

Both are required. Preferences can select the runtime explicitly; diagnostics list
searched/rejected paths. An older saved runtime preference can keep selecting an old
binary, so return to automatic detection after moving installations.

## A small game

This `main.lua` draws a movable square without requiring any assets:

```lua
local play = scene.new("play")
local player = {x = 40, speed = 180}

function game.load()
    input.bindAxis("move", {negative = {"a", "left"}, positive = {"d", "right"}})
    inspector.expose("Player.speed", player, "speed", {
        writable = true, min = 0, max = 600
    })
    scene.register(play)
    scene.switch("play")
end

function play:update(dt)
    player.x = player.x + input.axis("move") * player.speed * dt
end

function play:draw()
    graphics.clear(0.04, 0.05, 0.07)
    graphics.setColor(0.35, 0.8, 0.7)
    graphics.rectangle("fill", player.x, 100, 24, 24)
    graphics.setColor(1, 1, 1)
    graphics.print("A/D or arrows - edit Player.speed in Inspector", 20, 20)
end
```

Global game callbacks remain optional and run before scene callbacks. See
[scenes](docs/scenes.md), [input actions](docs/input-actions.md) and
[Live Inspector](docs/live-inspector.md) for lifecycle and write semantics.

## Create, run and export

```sh
cargo run -p kairo-cli -- new mygame --template top-down --title "My game"
cargo run -p kairo-cli -- check mygame
cargo run -p kairo-cli -- run mygame --profile development
cargo build --release --workspace -j 1
# Windows: use .\target\release\kairo.exe
./target/release/kairo build mygame --profile release --output exports/MyGame
```

Runtime profiles are not Cargo compiler profiles. Selecting **Release** in the
editor disables development defaults; it does not optimize an already-built debug
runtime. Select a release-built runtime before distribution. Export copies the host
runtime, `game/`, a package marker and notices; it does not cross-compile, sign or
bundle every system dependency. Players need no Rust installation, but must have
compatible system libraries/drivers. Test the relocated package on a clean machine.

The Windows native build/package helper is:

```powershell
.\tools\build-windows.ps1 -LowDisk -Package
```

It stops on failures and was not executable in the authoring environment. Source
integrity and Lua-library checks do not replace that native validation.

## Examples and starters

There are **23 examples and 16 embedded starters**. Every example includes readable
source; nontrivial controls are documented in its local README.

| Example | Main workflow / controls |
| --- | --- |
| `top-down` | Signal Yard: start with Enter, move with WASD/arrows or stick, collect eight charges, Escape pauses, B bookmarks. |
| `platformer` | One-way platform starter with input actions, jumping, particles and exposed movement values. |
| `scenes`, `input-actions` | Scene stack/lifecycle and configurable keyboard/mouse/controller actions. |
| `live-inspector` | Exposed scalar/vector/color/enum values and read-only fields. |
| `particles`, `audio-mixer` | Native emitter presets; keys 1/2/3 play through named buses, Space stops. |
| `replay-bugs` | B/F11 creates a pinned bug bookmark; inspect and restore through Replay. |
| `breakout`, `micro` | Playable arcade examples with restart and original assets. |
| `animation`, `tilemap` | Sprite-sheet clips and externally editable finite Tiled JSON. |
| `ui`, `link`, `replay` | Retained/debug UI, trusted remote iteration and explicit snapshot controls. |
| `hello-world`, `sprites`, `movement`, `audio`, `physics`, `gamepad`, `fonts`, `save-data` | Focused API examples. The fonts example uses bitmap fallback until you supply a licensed font. |

Original included PNG/WAV assets are MIT-licensed. No font files are bundled.

## Repository structure

```text
crates/kairo-core      Commands, configuration, input actions, inspector schema,
                      particles, physics, debug geometry, profiles, bookmark data
crates/kairo-assets    Texture/font caches and Tiled parsing
crates/kairo-render    wgpu batching, Micro presentation and debug UI painting
crates/kairo-audio     Kira playback resources and buses
crates/kairo-lua       Validated bindings, lifecycle, scenes, animator, game data
crates/kairo-project   Shared settings, templates, paths, search and packaging
crates/kairo-link      Versioned authenticated development transport
crates/kairo-replay    Bounded snapshot timeline and controlled RNG
crates/kairo-cli       Window/event loop, devices, control channel and commands
crates/kairo-editor    Project workspace, developer tools and supervised runtime
```

## Validation and contributing

```sh
python scripts/source_audit.py
python scripts/generate_api_docs.py --check
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features -j 1
cargo clippy --workspace --all-targets --all-features -j 1 -- -D warnings
cargo test --workspace --all-features -j 1
cargo build --workspace -j 1
python scripts/smoke.py
python scripts/package_smoke.py
```

`python scripts/lua_sanity.py` exercises native Lua libraries and deliberately
substituted engine boundaries. Its success does not certify Rust, GPU, physics,
audio, UI or network behavior. Real runtime smoke scripts require real executables
and fail if they are missing. [Release acceptance](docs/release-acceptance.md) covers
interactive and multi-machine checks.

## Documentation

Start with [getting started](docs/getting-started.md), [editor](docs/editor.md),
[CLI](docs/cli.md), [configuration](docs/configuration.md), [Lua guide](docs/lua-api.md)
and the [generated API reference](docs/api-reference.md).

3.4 guides: [scenes](docs/scenes.md), [input](docs/input-actions.md),
[inspection](docs/live-inspector.md), [animation states](docs/animation-state-machines.md),
[particles](docs/particles.md), [audio mixer](docs/audio-mixer.md),
[debug drawing](docs/debug-drawing.md), [maps](docs/tilemaps.md),
[profiles](docs/build-profiles.md), [prefabs](docs/prefabs.md),
[localization](docs/localization.md), [saves](docs/save-data.md).

Development features: [Link](docs/kairo-link.md), [Replay](docs/kairo-replay.md),
[Micro](docs/kairo-micro.md), [UI](docs/ui.md), [reload](docs/hot-reload.md),
[architecture](docs/architecture.md), [packaging](docs/packaging.md).

Read [CONTRIBUTING.md](CONTRIBUTING.md), [SECURITY.md](SECURITY.md),
[CHANGELOG.md](CHANGELOG.md) and [THIRD_PARTY.md](THIRD_PARTY.md).
The engine/editor and original examples use the [MIT license](LICENSE).
