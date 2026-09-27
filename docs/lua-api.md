# Lua API - Kairo2D 3.5.0

This is the implemented source API, not a claim of completed native validation.
See `VALIDATION.md` at the repository root. All listed functions have implementations. Unsupported future features are not
registered as empty stubs. Arguments are checked at the Rust boundary; invalid
arguments raise Lua errors that can be caught with `pcall`.

## 3.4 workflow APIs

The editor and [generated API reference](api-reference.md) share `lua-api.json`.
New subsystem guides: [scenes](scenes.md), [actions](input-actions.md),
[live/remote inspection](live-inspector.md), [animation states](animation-state-machines.md),
[particles](particles.md), [mixer](audio-mixer.md), [debug drawing](debug-drawing.md),
[profiles](build-profiles.md), [prefabs](prefabs.md), [localization](localization.md),
[save migrations](save-data.md), [tilemap additions](tilemaps.md), [bookmarks](kairo-replay.md).

`debug` now names a restricted Kairo module; Lua's native debug library is not exposed.
`game.replayRestored()` can reset transient state after supported snapshot restoration.
Scene callbacks use the same drawing phase as game.draw and cannot draw during update.

## Conventions

Coordinates use physical window pixels normally and logical canvas pixels in
Kairo Micro. Positive X is right and positive Y is down. Angles
are radians; positive sprite rotation looks clockwise on screen. Time is seconds.
Colors and volumes use the range `[0, 1]`, never `[0, 255]`. Positions, dimensions,
scales, colors and physics inputs must be finite.

Game paths are relative to the project root. Lua cannot request parent traversal
or absolute paths through the engine helpers. Resource values are opaque Lua
userdata, not numbers, raw pointers, or backend objects. Keep them in locals or
tables. Resource caches live for the session and are discarded on successful script reload. Textures can also be explicitly released.

## Lifecycle and events

```lua
function game.load() end
function game.update(dt) end
function game.draw() end

function game.keyPressed(key) end
function game.keyReleased(key) end
function game.mousePressed(x, y, button) end
function game.mouseReleased(x, y, button) end
function game.mouseMoved(x, y, dx, dy) end
function game.wheelMoved(dx, dy) end
function game.resized(width, height) end
function game.focusChanged(focused) end
function game.quit() end
function game.debugUI() end
function game.gamepadConnected(id, name) end
function game.gamepadDisconnected(id) end
function game.gamepadPressed(id, button) end
function game.gamepadReleased(id, button) end
```

Callbacks are optional; a present callback must be a function. `main.lua` executes
first, then `game.load` once. Each active tick processes replay controls and retained UI input, runs registered
native extensions and `game.update(dt)` plus the current scene update, advances fixed-step physics unless the top scene pauses it and optional
snapshots, then calls `game.draw()`, native draw callbacks, retained UI, profiler
overlay and `game.debugUI()`. A paused replay still draws. Window events arrive between frames. Polling
state is updated **before** the matching input callback. Keyboard repeats are
not new presses. Focus loss clears keys/buttons and synthesizes releases before
`focusChanged(false)`, preventing stuck movement.

Frame `dt` is clamped to 0.1 seconds. Physics steps at 1/120 second, at most twelve
steps per update; there is no render interpolation. Minimized zero-size windows
do not update or draw. Losing focus alone does not pause simulation.

F5 is reserved by the engine; F11 captures a bug bookmark when development tools are enabled. Closing the native
window exits; there is no quit-veto callback. `game.quit()` is called during orderly exit, but cannot cancel it. Wheel values use line-like
units; native pixel scroll events are divided by 40. Only nonzero resizes invoke
`game.resized`. Window size requests can be asynchronous.

## graphics

### Clear and color

```lua
graphics.clear(r, g, b, a)       -- a defaults to 1
graphics.setColor(r, g, b, a)    -- a defaults to 1
```

At the beginning of every draw callback the queue is empty, the clear color is
`(0.055, 0.067, 0.09, 1)`, the drawing color is white, and the camera is identity.
`clear` changes the background **and discards commands already queued this frame**.
`setColor` affects subsequent shapes, text and texture tint, not the background.
Set drawing state inside `game.draw`; values set during load/update will be reset
when drawing begins. RGB inputs are treated as sRGB and converted for linear
blending; alpha is straight alpha.

### Rectangles

```lua
graphics.rectangle("fill", x, y, width, height)
graphics.rectangle("line", x, y, width, height, lineWidth)
```

`x, y` is the top-left corner. Dimensions must be nonnegative; zero-size shapes
produce no drawing. Line width defaults to 1, must be positive, and is clamped
to half the smaller dimension. Outlines are four non-overlapping strips inside
the specified bounds. There is no rounded rectangle or general circle drawing API; the physics debug overlay can outline circle colliders.

### Textures and sprites

```lua
local texture = graphics.loadTexture("assets/player.png")
local width, height = texture:getDimensions()

graphics.draw(texture, x, y)
graphics.draw(texture, {
    x = 100, y = 100,
    rotation = 0,
    scale_x = 1, scale_y = 1,
    origin_x = 0, origin_y = 0,
    source = {x = 0, y = 0, width = 32, height = 32}
})
```

PNG and JPEG are supported. Decoding is synchronous; load outside the frame loop
when possible. The cache deduplicates canonical paths. The GPU texture is uploaded
lazily when first drawn. Repeated loads do not repeatedly read or upload the file.
Nearest-neighbor filtering is the default. `graphics.setFilter(texture, "linear")`
selects linear filtering; `"nearest"` restores pixel sampling. There are no mipmaps.
Filtering is per cached texture, not per sprite, and must be changed outside
`game.draw`. `graphics.releaseTexture(texture)` also runs outside draw and
invalidates all wrappers for that cached texture. Drawing a released handle is an
error. Loading that path again creates a fresh non-aliasing handle. `getDimensions`
returns cached dimensions even on a wrapper whose resource was released.
Same-size image reload preserves the handle; resized images require a session reload.

The positional overload requires both `x` and `y`. In the table overload, position
and origin default to zero, rotation to zero, and **each** scale axis to one.
A negative scale mirrors that axis. `source` is optional; when supplied it must
contain all four fields, positive dimensions, and remain inside the texture.
Without it, the whole texture is used. Origins are in unscaled source-rectangle
pixels, relative to that rectangle, not to the full image. Unknown table options
are errors.

The transform is `position + rotation * ((local - origin) * scale)`. Current camera
and tint are captured when each command is queued. Texture handles provide only
`getDimensions`; their internal IDs and GPU objects are not exposed. Distinct Lua
wrappers for the same cached texture need not compare equal with `==`.

### Bitmap text

```lua
graphics.print("Hello\nKairo2D", x, y, scale)
```

`scale` defaults to 2 and must be positive. Glyphs occupy an 8x8 grid, advance 8
pixels horizontally, and use a 10-pixel line advance, multiplied by scale. Newline
and tab are supported; tab advances 32 pixels times scale. This overload uses a built-in basic Latin bitmap font. TrueType/OpenType font
loading is available through the separate handle overload described below. Unsupported
characters display `?`. One print call accepts at most 4096 UTF-8 bytes. Glyph pixel
runs join the solid-shape batch rather than submitting separate GPU frames.

### Camera

```lua
graphics.setCamera(x, y, zoom, rotation) -- zoom=1, rotation=0
graphics.resetCamera()
local worldX, worldY = graphics.screenToWorld(screenX, screenY)
```

`x, y` is the world position mapped to the screen's top-left, not its center.
Zoom must be positive. The transform is
`screen = rotate(-rotation) * (world - cameraPosition) * zoom`.
`screenToWorld` uses the camera currently selected in Lua drawing state; select
the intended camera before using it during an input/update callback. The camera
resets before drawing and can be reset mid-frame for screen-space UI:

```lua
function game.draw()
    graphics.setCamera(200, 100, 2)
    graphics.rectangle("fill", 220, 120, 32, 32)
    graphics.resetCamera()
    graphics.print("Screen-space label", 12, 12)
end
```

`clear`, `rectangle`, `draw`, and `print` may only run inside `game.draw`. This
prevents commands submitted during update/load from being silently discarded.

### Viewport and scissor

```lua
graphics.setViewport(x, y, width, height)
graphics.resetViewport()
graphics.setScissor(x, y, width, height)
graphics.resetScissor()
```

These functions only run inside `game.draw`. All arguments are nonnegative
integers in game-canvas pixels (logical pixels in Micro). A viewport must have positive dimensions and
fit inside the current window. It maps drawing coordinate `(0,0)` to its top-left;
it does not automatically rescale a logical-resolution scene. Camera transforms
operate in that viewport. `screenToWorld` subtracts the active viewport origin
before undoing the camera transform.

Scissor is an absolute game-canvas-pixel clip rectangle, independent of camera/viewport
coordinates. It is intersected with the viewport; empty intersections discard
draws. Zero-width/height scissors intentionally hide subsequent drawing. Both
states reset at draw start; resetViewport restores the entire window and
resetScissor removes the extra clip. `clear` clears the entire surface and discards
previous draws, but preserves active camera/viewport/scissor state. State changes
split adjacent batches rather than reordering sprites.

## keyboard

```lua
local held = keyboard.isDown("space")
```

Names are case-insensitive and refer to physical key positions, not text input.
Letters use `a` through `z`; top-row digits use `0` through `9`. Arrow keys are
`left`, `right`, `up`, `down`. Other names include:

```text
space enter escape tab backspace delete insert home end pageup pagedown
lshift rshift lctrl rctrl lalt ralt lsuper rsuper
capslock numlock scrolllock f1 ... f12 (f5 is reserved)
- = [ ] \ ; ' , . / `
kp0 ... kp9 kp+ kp- kp* kp/ kp. kpenter
```

Unknown names return false. This API is for controls; composed text input, IME,
touch input and key rebinding are not implemented. Gamepads have their own module.

## mouse

```lua
local x, y = mouse.position()
local held = mouse.isDown(button)
```

Buttons are `1` left, `2` right, `3` middle, `4` back, `5` forward. Other numbers
are errors. Position is in game-canvas pixels and initially `(0, 0)` until cursor events
arrive. Micro maps window input through the letterboxed canvas; positions in the
bars can be negative or exceed the canvas dimensions. No pointer capture, cursor hiding, or relative mode is
implemented.

## window

```lua
local width, height = window.getSize()
local physicalWidth, physicalHeight = window.getPhysicalSize()
window.setTitle("My game")
window.setSize(1280, 720)
window.setVsync(true)
window.close()
```

Title must be nonblank and at most 256 UTF-8 bytes. Requested dimensions are
integers from 1 through 8192, subject to GPU and window-system constraints.
Mutations are queued and applied after the current successful frame. Do not
assume `getSize` changes immediately after `setSize`: use `game.resized`.
`getSize` reports logical canvas dimensions in Micro and window dimensions
otherwise; `getPhysicalSize` always reports physical window dimensions. Outside
Micro, `getSize` may report zero while minimized. OS window resizability is configured
through TOML. Fullscreen and display enumeration are not implemented.

## timer

```lua
local dt = timer.getDelta()
local seconds = timer.getTime()
local fps = timer.getFPS()
```

Delta is the current clamped frame duration. Time accumulates those deltas since
the current session started, not wall-clock time spent minimized or on an error
page. FPS is a smoothed reciprocal of nonzero unclamped wall-frame durations and begins
at zero; simulation deltas and measured frame durations are different concepts.
All three reset on reload. Headless execution advances at exactly 1/60 second
per update (subject to floating-point representation).

## audio

```lua
local available = audio.isAvailable()
local sound = audio.load("assets/chime.wav")
local seconds = sound:getDuration()

if available then
    local voice = audio.play(sound, {volume = 0.7, looping = false})
    audio.setVolume(voice, 0.3)
    audio.setMasterVolume(0.8)
    local playing = audio.isPlaying(voice)
    audio.stop(voice)
end

audio.stopAll()
```

WAV and Ogg/Vorbis decoding are enabled. `load` caches decoded samples by canonical
path and works without an output device. This is **not streaming**: whole sounds
are held in memory. `play` returns an independent voice; overlapping calls do not
share playback state. Options default to `volume=1`, `looping=false`; unknown
options are errors. Volumes are finite numbers from zero to one. Looping repeats
the full sound.

`stop` is idempotent. A stopped or completed voice reports false from `isPlaying`;
`setVolume` on a voice already removed from the manager raises an error. Finished
voices are pruned during updates and before new playback. Volume/stop changes
use Kira's default tween rather than abrupt parameter jumps. Master volume is
independent of per-voice volume. Sounds expose only `getDuration`; voices have
no Lua methods and are controlled through the module functions.

Without an output device, `isAvailable` is false and `play`/`setMasterVolume` raise
errors. They do not fabricate successful playback. There is no automatic device
reconnection. Reload stops every old voice and opens a fresh audio session. `audio.play` is
not allowed in top-level loading, `game.load`, or `game.restoreState`: queue the
first play from update or an input callback. This avoids a rejected reload
candidate starting audible playback. Loading sound data during load is supported.

## physics (experimental)

```lua
local floor = physics.newRectangle("static", 400, 500, 800, 24)
local box = physics.newRectangle("dynamic", 200, 100, 32, 32)
local ball = physics.newCircle("dynamic", 350, 100, 16)
physics.setGravity(0, 980)

local x, y = box:getPosition()
box:setPosition(200, 100)
local angle = box:getRotation()
box:setRotation(0.4)
local vx, vy = box:getVelocity()
box:setVelocity(0, -300)
box:applyImpulse(0, -20)
box:destroy()
```

Body positions are **centers**. Dimensions/radii must be positive. Only
`"static"` and `"dynamic"` are accepted. Velocities use pixels/second and gravity
uses pixels/second squared. Impulses use kg*pixel/second; they are divided by
`pixels_per_meter` before reaching Rapier. Default density is the solver's
1 kg/m^2; changing object dimensions changes mass. Colliders use friction 0.7,
restitution 0.2, and continuous collision detection is enabled on bodies.

The world steps automatically **after** `game.update` and **before** `game.draw`.
Do not manually integrate a physics body's position in addition to setting its
velocity. Setters wake affected bodies. No manual `physics.step` is exported.

Bodies remain in the world when a Lua variable is garbage-collected. Use
`destroy` to remove them; calling any method, including a second `destroy`,
after destruction raises an error. Reload destroys the whole world. There are
no joints, sensors, contact callbacks, raycasts, collision masks, material
setters, or polygon bodies in this release. Rapier types never cross into Lua.

## filesystem

```lua
local bytes = filesystem.read("assets/data.txt")
local present = filesystem.exists("assets/data.txt")
local names = filesystem.list(".")
```

`read` returns a binary-safe Lua string, limited to 64 MiB per file. Files must be
regular files. `exists` returns false for a missing path; invalid paths and other
I/O failures are errors. `list` returns a sorted, one-based array of UTF-8 child
names, including directories. Symlinks resolving outside the root are excluded
from listings and cannot be read. This module is read-only. The separate `save` module provides scoped JSON storage;
it does not write into project files.

These checks are guardrails for trusted projects, not race-free OS sandboxing.
A filesystem that changes between path validation and open can introduce a
check/use race. Do not run hostile projects with access to valuable local data.

## assets

```lua
local stats = assets.stats()
-- stats.textures, stats.texture_bytes
-- stats.sounds, stats.sound_bytes, stats.voices, stats.bodies
```

Counts cover the current session. Byte counts are decoded CPU data, not total
RAM/VRAM. Texture loading is under `graphics`; sound loading is under `audio`.
There is no second competing asset-loading API. `graphics.releaseTexture` unloads a texture; sounds remain cached until session replacement.

## Limits and standard Lua libraries

The engine caps the Lua allocator at 128 MiB, input files at 64 MiB, textures at
8192 pixels per dimension and 64 MiB decoded each, texture cache at 256 MiB and
4096 items, decoded sound cache at 256 MiB and 1024 items, tracked voices at 512,
and physics bodies at 10,000. Frames permit 50,000 queued commands and 250,000
expanded quads. These are guardrails, not a hard process memory budget. In
particular, sound decoding allocates before its decoded-size check.

The base library plus table, string, math, utf8, coroutine and package libraries
are available. `os`, `io`, `debug`, `dofile`, `loadfile`, and native module loading
are not exposed. `require` only searches project Lua sources or preloaded modules.
`print(...)` calls Lua `tostring` on each argument, joins them with tabs and logs
an INFO message capped at 8192 UTF-8 bytes. It reaches the editor console and a
connected Link host through the runtime logging channel. Embedded newlines are
preserved. Native messages include level/category; the editor captures stdout
and stderr. Do not log credentials or private data to a remote development peer.

The main Lua thread has a two-second instruction watchdog per engine callback,
checked every 10,000 instructions. Native calls are not preempted, and this is not
a hostile-code execution deadline. Coroutine behavior and native dependencies
are further reasons not to consider the runtime a security sandbox.

`kairo check` skips symlink entries and limits traversal to 64 directory levels and
10,000 Lua source files. It compiles sources without executing them.

## Contact polling

```lua
local touching = body:isTouching(otherBody)
```

Returns contact state from the most recent physics step; immediately created or
teleported bodies are not queried against the world until it advances. Since
physics advances after update, `game.update` sees the previous completed step,
while draw sees the current step. Comparing a body to itself returns false;
foreign/destroyed bodies raise errors. This is not a collision callback or a
predictive overlap query, and no backend contact-manifold data escapes to Lua.


## TrueType/OpenType fonts

```lua
local font = graphics.loadFont("assets/font.ttf", 24)
local width, height = graphics.measureText(font, "Hello")
-- In game.draw:
graphics.print(font, "Hello", 20, 20)
-- Outside game.draw, when no UI or draw call still needs the font:
graphics.releaseFont(font)
```

Supported fonts are TTF/OTF outlines that fontdue can parse, cached by canonical
path and pixel size. Size is 4..256. Text is positioned from the top-left line box;
newlines, tabs and basic kerning work. Drawing uses current camera/color. Measure
uses the same layout and can populate the glyph cache. Font wrappers are opaque.
One font call is limited to 4096 characters and 16 KiB of UTF-8; 32 cached fonts,
8192 glyphs/font and eight 1024-square atlas pages/font are allowed within the
shared texture budget. Releasing a font invalidates its wrappers and releases its
atlas pages. No shaping, bidi, rich text, automatic wrapping or font fallback is
promised. No system font files are redistributed. See [fonts](fonts.md).

## animation

```lua
local walk = animation.new(texture, {
    {x=0, y=0, w=16, h=16, duration=0.1},
    {x=16, y=0, w=16, h=16, duration=0.1}
})
walk:setLooping(true)
walk:setSpeed(1.0)
walk:update(dt) -- Explicit: call during game.update.
walk:pause()
walk:resume()
walk:restart()
local finished, frame = walk:isFinished(), walk:getFrame()
-- In game.draw:
graphics.draw(walk, x, y)
```

`animation.load("assets/walk.anim.json")` loads versionless JSON metadata containing
texture, looping and frames; texture is relative to the metadata file. There are
1..4096 in-bounds positive frame rectangles, each with a finite duration from one microsecond through one hour.
Clips loop by default. Speed is 0..100; zero freezes the clock. One-shot playback
holds the last frame and reports completion. Frame indices are one-based. Instances
have their own clock; textures remain cached/shared. `graphics.draw` accepts the
same transform options as textures, but an animation's current rectangle overrides
an explicit `source`. Replay does not automatically capture animation clocks.
See [animations](animations.md).

## tilemap

```lua
local map = tilemap.load("assets/world.tmj")
local columns, rows, tileWidth, tileHeight = map:getSize()
local layers = map:getLayers()
local gid = map:getTile("Ground", 0, 0)
map:setLayerVisible("Ground", true)
local objects = map:getObjects()
-- In game.draw, using current camera and tint:
map:draw(0, 0)
```

Only finite, orthogonal Tiled JSON with integer-array data is implemented, including
embedded tilesets and external TSJ sheets. Tilesets must have uniform tiles matching
the map cell size. Layers preserve visibility, opacity and pixel offsets; drawing
culls against the active camera/viewport. Horizontal/vertical flips work; diagonal
flips, LDtk/TMX, compressed data, infinite maps and group layers are rejected.
Tile coordinates are zero-based; lookup returns the raw GID, including flip bits,
zero for an empty cell, and nil for coordinates outside a named layer. Unknown
layer names are errors. Objects expose id, name, type, x/y, width/height and rotation
(in **Tiled degrees**, unlike sprite radians); they are data, not automatic physics.
Map caches deduplicate loading; visibility overrides are per wrapper. See [tilemaps](tilemaps.md).

## gamepad

```lua
local ids = gamepad.connected()
for _, id in ipairs(ids) do
    local name = gamepad.name(id)
    local jump = gamepad.isDown(id, "a")
    local x, y = gamepad.axis(id, "left_stick")
    local trigger = gamepad.axis(id, "right_trigger")
end
```

`connected` returns a sorted sequence of connection IDs. IDs start at one but are
not necessarily contiguous, persistent or equal to player numbers. Name is nil
when disconnected. Unknown button/axis names are errors; disconnected polling
returns false or zero. Button names: a/b/x/y, start/back/guide, left_shoulder,
right_shoulder, left_trigger, right_trigger, left_stick, right_stick, up/down/left/right.
Axis names: left_stick, right_stick, left_trigger, right_trigger. Sticks return x/y
in [-1,1] after radial deadzone 0.15, with positive Y down. Triggers return value in
[0,1] and a second zero result. The backend emits the gamepad lifecycle callbacks
listed above. Headless mode has no controller backend; unavailable hardware is not
simulated. See [gamepads](gamepads.md).

## save

```lua
local settings = save.read("settings.json") or {volume=0.8}
-- From update, input or quit, not game.load:
save.write("settings.json", settings)
local present = save.exists("settings.json")
local removed = save.remove("settings.json")
```

Set a stable `[save].identity` in kairo.toml. New-project templates receive a unique
identity on creation; preserve it across exports and game updates. Storage uses a
per-game platform application-data folder, separate from project assets. Names
must be simple ASCII `.json` filenames, not directory paths. Files are limited to
4 MiB; writes stage then publish atomically. `read` returns nil when missing and
raises on invalid JSON/I/O errors; remove returns whether a file was removed.
Values follow mlua's JSON serialization rules: use plain string-keyed objects or
sequences, finite scalar values, and no functions/userdata/cycles. JSON is not a
lossless serializer for every possible Lua table. Writes/removes are blocked
while loading a replacement session. Save changes are not undone by replay.
See [save data](save-data.md).

## profiler

```lua
profiler.show(true)
local p = profiler.stats()
-- p.fps, frame_ms, tick_ms, update_ms, draw_ms, physics_ms,
-- game_ms, scene_ms, ui_ms, audio_ms, render_ms,
-- sprites_submitted, active_bodies, active_colliders,
-- active_particles, audio_voices, loaded_audio, textures,
-- lua_callbacks, callback_samples, callback_samples_dropped
```

`frame_ms` is the supplied frame interval; `tick_ms` measures simulation CPU wall
time. The phase timings can overlap: scene dispatch is included inside update or
draw, and UI drawing is included inside draw. `render_ms` measures the renderer
call from the windowed runtime. None of these are GPU timestamps. Renderer
counters can lag the current Lua callback by one presented frame. Texture bytes
estimate decoded CPU storage, not total VRAM/RAM. The callback list records at
most 128 distinct callbacks during the frame and sends the 32 with the highest
total time to Editor; `callback_samples_dropped` counts omitted calls. Each
callback entry has `script`, `scene`, `node_path`, `callback`, `calls`, `total_ms`
and `max_ms`. Divide total by calls for the average. Callback timing is enabled
by the Debug build profile or an explicit profile override. `profiler.show`
controls the runtime bitmap overlay; `debug.showProfiler` is an alias. Lua's
standard debug library is absent. See [profiling](profiler.md).

## ui and debugui

See [UI](ui.md) for exact options, interaction rules and examples. The public
retained constructors are `ui.panel`, `ui.label`, `ui.button`, `ui.image` and
`ui.progress`. Panels own children through `add`; widgets expose setText,
setValue, setVisible and setEnabled. Module functions are setFont, clear, remove
and wantsMouse. The runtime updates/draws the tree automatically; do not call
underscore-prefixed internal hooks. Widgets are Lua objects, not backend pointers.

`debugui.window(title, callback)`, text, button, checkbox and slider may be used
only inside `game.debugUI`; native responses are returned on the next callback.
`debugui.wantsMouse()` reports the prior egui pointer-capture decision. This is an
immediate development overlay, not the shipped game-UI widget tree.

## replay and random

```lua
local player = {x=40, y=80, health=3}
function game.load()
    replay.register("player", player)
    replay.enable(true)
    random.seed(123)
end
function game.keyPressed(key)
    if key == "r" then replay.rewind(2.0) end
    if key == "space" then replay.resume() end
end
```

Replay supports register, enable, capture, clear, stats, rewind, seek, pause,
resume and step. Rewind takes nonnegative seconds; seek uses the one-based index
in `replay.stats().snapshots`. Controls apply at a frame boundary, with only the
most recent pending request retained. Seek/rewind pauses; resume/step branches.
Stats contain recording, paused, bytes, frame, time and snapshot metadata.

Supported registered state is finite numbers, booleans, strings and plain tables
with scalar keys. No functions, userdata, metatables, shared table aliases or
cycles. Roots must be tables. Capture also stores Kairo RNG, polled input and
limited physics-body state; physics topology must match on restore. Solver caches,
audio positions, unregistered locals/globals, UI, files and network effects are not
rewound. This is **not** automatic deterministic time travel. See
[Replay](kairo-replay.md) for limits and editor controls.

`random.seed(nonnegative_integer)` sets the Kairo RNG. `random.float()` returns
[0,1); `random.integer(min,max)` returns an inclusive integer range. This stream is
reproducible and replayable, not cryptographically secure. Lua's math.random is
unchanged and not captured.

## Reload state hooks and Kairo Link

Optional `game.saveState()` returns data-only transferable state; when it exists,
the replacement must implement `game.restoreState(data)`. The replacement first
loads main.lua and game.load, then receives the decoded state. Data uses the same
strict plain-table codec as Replay and an additional 8 MiB transfer limit.
Unsupported values or a failing restore hook reject the candidate; the previous
session remains active. Without hooks, successful Lua replacement resets game
state. These hooks apply both to local full-session reload and Link revisions.

There is no general networking or remote-shell Lua module. Link is opt-in
editor/runtime development transport with encrypted authenticated connections;
see [Link](kairo-link.md) for pairing, project transfer limits and threat model.
The protocol does not synchronize gameplay or guarantee seamless arbitrary code
changes. A candidate can still have bugs in later callbacks even if load/restore
succeeded; those errors are reported normally.

## Kairo Micro and extension boundary

Micro is configured in kairo.toml, not enabled through an invented Lua module.
Graphics/input use its logical canvas, followed by integer nearest presentation
and optional built-in palette quantization. See [Micro](kairo-micro.md).
The Rust `NativeExtension` host boundary is experimental and compile-time; it is
not a dynamic plugin loader, C#/C++ runtime or WASM implementation. See
[extensions](extensions.md).
