# Project configuration

`kairo.toml` is optional; `main.lua` is the fixed entrypoint. Unknown keys are tolerated
so tools can keep their own tables. The editor preserves unknown fields/comments
when changing supported options and refuses to overwrite an externally changed file.

The defaults are:

```toml
[game]
title = "Kairo2D"
width = 960
height = 540
vsync = true
resizable = true

[audio]
enabled = true

[development]
hot_reload = true
tools = true
profiler = false
link = true
log_level = "info"

[physics]
gravity_x = 0.0
gravity_y = 980.0
pixels_per_meter = 100.0

[save]
identity = ""

[replay]
enabled = false
seconds = 10.0
frequency = 30
memory_mib = 32

[micro]
enabled = false
width = 320
height = 180
integer_scaling = true
pixel_snap = true
palette = "kairo16"
```

Window dimensions are integers 1..8192 and remain subject to device/OS limits.
Title must be nonblank and at most 256 UTF-8 bytes. Vsync selects the backend's
automatic vsync/no-vsync presentation mode. No fullscreen or custom entrypoint key
is consumed. Physics values must be finite; pixels_per_meter must be positive.
Gravity and body positions/velocities are presented in game pixels and seconds.

Save identity is empty (saves disabled until requested) or 1..96 ASCII letters,
digits, hyphens and underscores. New projects get a unique `game-...` identity.
Changing it selects a different save directory. Do not change it per launch or
when renaming a game. Examples use stable, example-specific identities.

Replay seconds must be 0.1..120, capture frequency 1..120 Hz and memory 1..256 MiB.
Actual history can be shorter when the memory budget is hit. Encoded snapshot bytes
are bounded; transient allocations and other engine memory are additional.

Micro dimensions are 1..2048. Palettes are `kairo16`, `grayscale`, `olive4` or `none`.
The first three are original predefined RGB palettes; `none` disables quantization.
The profile is configured at window creation, not switchable dynamically. Window
size remains physical; game dimensions and input positions become canvas pixels.

Local configuration edits require Stop/Run, not just a script reload. Link revisions
must retain the receiver's save identity and Micro configuration; incompatible
changes are rejected rather than silently changing storage or GPU configuration.

Link does not open a listener because of a config file. Hosting requires an explicit
editor action or CLI subcommand; connecting requires explicit CLI flags and a token
file. This prevents a downloaded project from enabling remote development by itself.

## Runtime profiles

`--profile development|debug|release|micro` (or the editor toolbar) applies profile
defaults after base settings, followed by `[profile.NAME]` overrides. Recognized
profile keys are strict; unknown names/override keys are rejected. Top-level unknown
custom tables are still preserved. Supported overrides: hot_reload, tools, profiler,
link, replay, micro, width, height, vsync and log_level. Dimensions override the
physical window, not the Micro canvas.

`development.tools=false` gates developer commands, Inspector writes, debug UI/geometry
and bookmarks. `profiler` sets initial overlay visibility. `link` is permission to
join a session, not an automatic listener. `log_level` accepts error/warn/info/debug/
trace/off; an explicit RUST_LOG environment override takes precedence.

Release defaults turn off tools/Link/hot reload/profiling/replay and lower logging to
warn; explicit profile overrides can re-enable them. The game-facing replay API can
still implement intentional gameplay rewind. No compiler optimization or code stripping
is controlled by these keys. See [build profiles](build-profiles.md).

Input mappings and particle presets are separate TOML documents. Edit `input.toml`
through Input Mappings, or see [input actions](input-actions.md).
