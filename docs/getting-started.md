# Getting started

Kairo2D is distributed here as source code. Build it on your computer before running
the editor or examples. The recorded build and test results are in
[VALIDATION.md](../VALIDATION.md).

Install the prerequisites listed in the root README. From the repository root, run
`cargo fmt --all` and `cargo build --workspace -j 1` to build both the editor and
runtime. Open a project in the editor with `cargo run -p kairo-editor`, or run an
example directly with `cargo run -p kairo-cli -- run examples/hello-world`. For a
game that uses several systems together, try `examples/top-down` (Signal Yard).

In the editor choose a new project directory inside an existing parent, a title,
and a template. The directory must not already exist. Projects include `main.lua`,
`kairo.toml` and `assets/`; most templates use only a few readable Lua files.
The generated save identity is stable and independent of the project title.

```sh
cargo run -p kairo-cli -- new mygame --template movement --title "My game"
cargo run -p kairo-cli -- check mygame
cargo run -p kairo-cli -- run mygame
```

`main.lua` defines optional callbacks on the pre-created `game` table. Keep resource
handles in locals or tables, load assets once, update gameplay in `game.update`,
and queue drawing in `game.draw`. Errors retain filenames and callback context.

```lua
local texture
local x = 32
function game.load() texture = graphics.loadTexture("assets/player.png") end
function game.update(dt)
    if keyboard.isDown("right") then x = x + 100 * dt end
end
function game.draw()
    graphics.clear(0.04, 0.05, 0.07)
    graphics.draw(texture, x, 120)
end
```

The movement template supplies the example image. In an empty project, create a PNG
with **Assets > New sprite** or copy an image you have permission to use. You can
always draw a rectangle without an asset.

Save files before running. F5 runs; Shift+F5 stops. Local Lua edits trigger a fresh
candidate session and swap only if its entrypoint/load hooks succeed. Optional
`game.saveState`/`restoreState` preserve supported plain Lua data. Future update/draw
errors are not statically predictable; read [hot reload](hot-reload.md).

When Run reports a missing runtime, inspect the complete path diagnostics. Select
`kairo.exe` in Preferences, never the editor. Release installations need
`Kairo.exe` plus `bin/kairo.exe`. A runtime that failed to link cannot be discovered.
Build artifacts may live outside `target/` if `CARGO_TARGET_DIR` is configured.

For distribution, use [building games](building-games.md). For a GUI-independent
check after building: `python scripts/smoke.py --frames 60`.

## New game-building tools

Choose the Top-Down, Platformer, Scene, Input Mapping, Live Inspector, Particle,
Audio Mixer or Replay Bug starter to try the new systems. Input Mappings writes
input.toml; Particles previews/saves emitter presets; Inspector edits only explicitly
writable fields. Run in Development or Debug to use developer controls.

Start with [scenes](scenes.md), [input actions](input-actions.md) and
[Live Inspector](live-inspector.md). The feature bar and Ctrl/Cmd+Shift+P palette
provide direct access; no hidden startup command is needed for local tools.
