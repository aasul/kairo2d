# API signatures and snippets

Generated from `docs/lua-api.json`. The editor uses the same metadata offline.
Methods headed Animator, Emitter and TileMap refer to returned objects, not global modules.
Snippets illustrate signatures; variables such as player/map/sparks must exist in your game.

## `graphics.clear(r, g, b, a?)`

Clear the frame; color channels are in 0..1.

```lua
graphics.clear(0.07, 0.08, 0.10)
```

## `graphics.setColor(r, g, b, a?)`

Set tint for following draw commands.

```lua
graphics.setColor(1, 1, 1, 1)
```

## `graphics.rectangle(mode, x, y, width, height, lineWidth?)`

Mode is fill or line; units are pixels.

```lua
graphics.rectangle("fill", 100, 100, 64, 64)
```

## `graphics.loadTexture(path)`

Decode and cache an image, returning an opaque handle.

```lua
graphics.loadTexture("assets/sprite.png")
```

## `graphics.draw(texture, x, y)`

Draw with the current tint and camera.

```lua
graphics.draw(texture, 100, 100)
```

## `graphics.draw(texture, options)`

Supports origin_x/origin_y and a source region.

```lua
graphics.draw(texture, { x = 100, y = 100, rotation = 0, scale_x = 1, scale_y = 1 })
```

## `graphics.print(text, x, y, scale?)`

Draw built-in bitmap text.

```lua
graphics.print("Hello Kairo", 24, 24, 2)
```

## `graphics.setCamera(x, y, zoom?, rotation?)`

Set camera transform for following draws.

```lua
graphics.setCamera(0, 0, 1, 0)
```

## `graphics.resetCamera()`

Restore the screen-space camera.

```lua
graphics.resetCamera()
```

## `graphics.screenToWorld(x, y)`

Convert screen coordinates using the current camera.

```lua
graphics.screenToWorld(0, 0)
```

## `graphics.worldToScreen(x, y)`

Convert world coordinates with the current camera and viewport.

```lua
local sx, sy = graphics.worldToScreen(player.globalPosition.x, player.globalPosition.y)
```

## `graphics.setViewport(x, y, width, height)`

Set a pixel viewport for following commands.

```lua
graphics.setViewport(0, 0, 320, 180)
```

## `graphics.resetViewport()`

Restore the full-window viewport.

```lua
graphics.resetViewport()
```

## `graphics.setScissor(x, y, width, height)`

Clip subsequent draws in window pixel coordinates.

```lua
graphics.setScissor(16, 16, 200, 120)
```

## `graphics.resetScissor()`

Disable scissor clipping.

```lua
graphics.resetScissor()
```

## `graphics.setFilter(texture, mode)`

Choose nearest or linear sampling outside game.draw.

```lua
graphics.setFilter(texture, "nearest")
```

## `graphics.releaseTexture(texture)`

Release outside game.draw; all copies of the cached resource handle become invalid.

```lua
graphics.releaseTexture(texture)
```

## `audio.isAvailable()`

Check whether audio playback is available.

```lua
audio.isAvailable()
```

## `audio.load(path, options?)`

Decode/cache WAV or OGG; optional {bus="sfx"} sets this wrapper default.

```lua
audio.load("assets/sound.wav")
```

## `audio.play(sound, options?)`

Voice options: volume, looping, bus, pitch (speed ratio), pan (-1..1). Playback is forbidden during candidate loading.

```lua
audio.play(sound, { volume = 0.5, looping = false })
```

## `audio.stop(voice)`

Stop one playback voice.

```lua
audio.stop(voice)
```

## `audio.stopAll()`

Stop all playback voices.

```lua
audio.stopAll()
```

## `audio.setVolume(voice, volume)`

Set voice volume in 0..1.

```lua
audio.setVolume(voice, 0.5)
```

## `audio.setMasterVolume(volume)`

Set master volume in 0..1.

```lua
audio.setMasterVolume(0.5)
```

## `audio.isPlaying(voice)`

Check whether a voice remains active.

```lua
audio.isPlaying(voice)
```

## `keyboard.isDown(key)`

Poll normalized physical key names.

```lua
keyboard.isDown("space")
```

## `mouse.position()`

Return current mouse x/y in game-canvas pixels (logical in Micro).

```lua
mouse.position()
```

## `mouse.isDown(button)`

Poll button 1=left, 2=right, 3=middle.

```lua
mouse.isDown(1)
```

## `window.close()`

Request a normal game-window shutdown.

```lua
window.close()
```

## `physics.newRectangle(kind, x, y, width, height)`

Create a box body, positioned at its centre.

```lua
physics.newRectangle("dynamic", 100, 100, 32, 32)
```

## `physics.newCircle(kind, x, y, radius)`

Create a circular body.

```lua
physics.newCircle("dynamic", 100, 100, 16)
```

## `physics.setGravity(x, y)`

Set gravity in pixels per second squared.

```lua
physics.setGravity(0, 980)
```

## `graphics.loadFont(path, size)`

Load a TTF/OTF font and cache it by path and pixel size.

```lua
local font = graphics.loadFont("assets/font.ttf", 24)
```

## `graphics.print(font, text, x, y)`

Queue text using a cached font; call from game.draw.

```lua
graphics.print(font, "Hello Kairo", 20, 20)
```

## `graphics.measureText(font, text)`

Measure text with the same glyph layout used for drawing.

```lua
local width, height = graphics.measureText(font, "Hello")
```

## `graphics.releaseFont(font)`

Release a cached font outside game.draw; invalidates its handles.

```lua
graphics.releaseFont(font)
```

## `animation.new(texture, frames)`

Create an independent sprite-animation clock.

```lua
local walk = animation.new(texture, {
    {x = 0, y = 0, w = 16, h = 16, duration = 0.1},
    {x = 16, y = 0, w = 16, h = 16, duration = 0.1}
})
```

## `animation.load(path)`

Load animation metadata; its texture path is metadata-relative.

```lua
local walk = animation.load("assets/walk.anim.json")
```

## `tilemap.load(path)`

Load finite orthogonal Tiled JSON, not LDtk or TMX.

```lua
local map = tilemap.load("assets/world.tmj")
```

## `gamepad.connected()`

Get connected IDs; IDs are not necessarily contiguous.

```lua
local controllers = gamepad.connected()
```

## `gamepad.name(id)`

Get a connected controller name, or nil.

```lua
local name = gamepad.name(id)
```

## `gamepad.isDown(id, button)`

Poll a controller button using Kairo names.

```lua
local jump = gamepad.isDown(id, "a")
```

## `gamepad.axis(id, axis)`

Poll a stick with deadzone or trigger value; positive Y is down.

```lua
local x, y = gamepad.axis(id, "left_stick")
```

## `save.read(name)`

Read JSON from the configured per-game save directory.

```lua
local settings = save.read("settings.json") or {volume = 0.8}
```

## `save.write(name, value)`

Write JSON atomically outside game.load/reload hooks.

```lua
save.write("settings.json", {volume = 0.8})
```

## `save.exists(name)`

Check a scoped JSON save name.

```lua
local exists = save.exists("settings.json")
```

## `save.remove(name)`

Remove one scoped save, outside game.load.

```lua
local removed = save.remove("settings.json")
```

## `profiler.show(enabled)`

Enable the runtime CPU/counter overlay.

```lua
profiler.show(true)
```

## `profiler.stats()`

Read CPU phase timings and most recent render counters.

```lua
local sample = profiler.stats()
```

## `random.seed(seed)`

Seed Kairo reproducible RNG, not cryptographic randomness.

```lua
random.seed(123)
```

## `random.float()`

Get a replayable number in [0,1).

```lua
local value = random.float()
```

## `random.integer(min, max)`

Get an inclusive replayable integer range.

```lua
local value = random.integer(1, 6)
```

## `replay.register(name, state)`

Register a plain data-only table; clears snapshot history.

```lua
replay.register("player", player_state)
```

## `replay.enable(enabled)`

Enable automatic bounded snapshot recording.

```lua
replay.enable(true)
```

## `replay.capture()`

Capture registered data, input, RNG and supported body state.

```lua
replay.capture()
```

## `replay.clear()`

Discard snapshot history.

```lua
replay.clear()
```

## `replay.stats()`

Read frame, time, byte count and retained snapshot metadata.

```lua
local history = replay.stats()
```

## `replay.rewind(seconds)`

Queue rewind to an earlier snapshot and pause.

```lua
replay.rewind(2.0)
```

## `replay.seek(index)`

Queue seek to a one-based snapshot index.

```lua
replay.seek(1)
```

## `replay.pause()`

Pause simulation at a frame boundary; drawing continues.

```lua
replay.pause()
```

## `replay.resume()`

Resume and discard history from the abandoned future.

```lua
replay.resume()
```

## `replay.step()`

Advance one simulation tick while remaining paused.

```lua
replay.step()
```

## `ui.panel(options)`

Create an automatically drawn retained UI root panel.

```lua
local panel = ui.panel({x = 20, y = 20, width = 260, height = 160, padding = 12})
```

## `ui.label(text, options)`

Add a text widget to an existing panel.

```lua
local label = panel:add(ui.label("Score: 0"))
```

## `ui.button(text, callback, options)`

Add a pointer-operated retained button.

```lua
panel:add(ui.button("Start", function() print("Start") end))
```

## `ui.image(texture, options)`

Draw a texture inside a UI layout.

```lua
panel:add(ui.image(texture, {width = 64, height = 64}))
```

## `ui.progress(value, options)`

Add a progress bar clamped to 0..1.

```lua
panel:add(ui.progress(0.5, {height = 16}))
```

## `ui.setFont(font)`

Choose the default retained UI font handle.

```lua
ui.setFont(font)
```

## `ui.remove(widget)`

Detach a UI widget from its parent/root list.

```lua
ui.remove(widget)
```

## `ui.clear()`

Remove all retained UI roots and pending pointer state.

```lua
ui.clear()
```

## `ui.wantsMouse()`

Determine whether the pointer is inside a visible UI root.

```lua
local over_ui = ui.wantsMouse()
```

## `debugui.window(title, callback)`

Queue a native development window inside game.debugUI only.

```lua
debugui.window("Player", function()
    speed = debugui.slider("Speed", speed, 0, 500)
end)
```

## `debugui.text(text)`

Add text inside a debugui window.

```lua
debugui.text("Runtime controls")
```

## `debugui.button(label)`

Read a queued button response from the previous native frame.

```lua
if debugui.button("Reset") then speed = 120 end
```

## `debugui.checkbox(label, value)`

Return the current queued checkbox value.

```lua
enabled = debugui.checkbox("Enabled", enabled)
```

## `debugui.slider(label, value, min, max)`

Return a finite range-clamped queued slider value.

```lua
speed = debugui.slider("Speed", speed, 0, 500)
```

## `debugui.wantsMouse()`

Read the previous native pointer-capture decision.

```lua
local captured = debugui.wantsMouse()
```

## `window.getSize()`

Get logical Micro dimensions or regular physical window dimensions.

```lua
local width, height = window.getSize()
```

## `window.getPhysicalSize()`

Get physical window dimensions regardless of Micro mode.

```lua
local width, height = window.getPhysicalSize()
```

## `window.setSize(width, height)`

Queue a physical window-size request.

```lua
window.setSize(1280, 720)
```

## `window.setTitle(title)`

Queue a native window title change.

```lua
window.setTitle("My Kairo Game")
```

## `window.setVsync(enabled)`

Queue vsync configuration.

```lua
window.setVsync(true)
```

## `timer.getDelta()`

Read the current clamped simulation delta.

```lua
local dt = timer.getDelta()
```

## `timer.getTime()`

Read current session simulation time.

```lua
local seconds = timer.getTime()
```

## `timer.getFPS()`

Read smoothed wall-frame FPS.

```lua
local fps = timer.getFPS()
```

## `filesystem.read(path)`

Read a project-scoped file as a binary-safe Lua string.

```lua
local text = filesystem.read("assets/data.txt")
```

## `filesystem.exists(path)`

Check a project-scoped path.

```lua
local present = filesystem.exists("assets/player.png")
```

## `filesystem.list(path)`

Return sorted project directory child names.

```lua
local names = filesystem.list("assets")
```

## `assets.stats()`

Read texture/audio/body counts and decoded memory estimates.

```lua
local assets_used = assets.stats()
```

## `scene.new(name)`

Create a scene table with a validated node graph and root; existing scene callbacks still work.

```lua
local play = scene.new("play")
```

## `scene.load(path)`

Load a version 1 JSON scene hierarchy from a project-relative .scene file.

```lua
local level = scene.load("scenes/level.scene")
```

## `scene.createNode(scene, type, name)`

Create a typed child of the scene root and return a validated node reference.

```lua
local world = level:createNode("Node2D", "World")
```

## `node:createChild(type, name)`

Create an ordered child node; cycles, invalid names and stale references are rejected.

```lua
local player = world:createChild("Sprite", "Player")
```

## `node:setProperty(name, value)`

Set a serialized node property; supported scene node types read native render, camera, physics and audio properties.

```lua
sprite:setProperty("texture", "assets/player.png")
```

## `node:getProperty(name)`

Read a serialized property or nil; dynamic body velocity is updated after the physics step.

```lua
local velocity = player:getProperty("velocity")
```

## `node:removeProperty(name)`

Remove one node property; returns whether the property existed.

```lua
sprite:removeProperty("texture")
```

## `node.position / node.globalPosition`

Set a local 2D position and read the cached position inherited from ancestors.

```lua
player.position = {x=100, y=200}; local x = player.globalPosition.x
```

## `node.rotation / node.scale / node.pivot`

Edit finite local transforms; node.worldTransform exposes the resulting affine matrix.

```lua
player.rotation = math.rad(30); player.scale = {x=2, y=2}
```

## `node:reparent(parent, index?)`

Move a node under another parent at a 1-based sibling index; cycles are rejected.

```lua
player:reparent(world, 1)
```

## `node:duplicate() / node:destroy()`

Copy or remove a subtree; stale node references always fail safely.

```lua
local copy = player:duplicate(); player:destroy()
```

## `node:on(event, callback)`

Subscribe to an event emitted by this node. The subscription is removed when the node is destroyed.

```lua
local token = player:on("hit", function(amount) health = health - amount end)
```

## `node:emit(event, ...)`

Dispatch a node event synchronously, passing all supplied Lua values to subscribers.

```lua
player:emit("hit", 10)
```

## `node:off(token)`

Disconnect a signal token. Returns false if it was already removed.

```lua
player:off(token)
```

## `Events.on(event, callback, owner?)`

Subscribe to a global event; optional node ownership disconnects automatically on destruction.

```lua
local token = Events.on("round_end", function() showScore() end, hud)
```

## `Events.emit(event, ...)`

Dispatch a global event synchronously. Subscriber changes during dispatch are safe.

```lua
Events.emit("round_end", score)
```

## `Events.off(token)`

Disconnect a global or node signal token.

```lua
Events.off(token)
```

## `scene.attachScript(scene, node, path)`

Attach a Lua module table with per-node state and explicit lifecycle callback registration.

```lua
local state = level:attachScript(player, "scripts/player.lua")
```

## `scene.register(scene)`

Register one named scene. Duplicate names and invalid callbacks are errors.

```lua
scene.register(play)
```

## `scene.switch(name)`

Queue replacement of the scene stack by name or scene table at a callback boundary.

```lua
scene.switch("play")
```

## `scene.push(name)`

Suspend the current top and push an overlay; the top alone receives updates/input.

```lua
scene.push("pause")
```

## `scene.pop()`

Pop and leave the top scene, then resume the previous one. Cannot pop the last scene.

```lua
scene.pop()
```

## `scene.reload()`

Leave and re-enter the current scene without reloading source or globals.

```lua
scene.reload()
```

## `scene.current()`

Current scene name, or nil. scene.shared holds ordinary persistent Lua data.

```lua
local name = scene.current()
```

## `scene.info()`

Registered names, current name and active stack as plain data.

```lua
local info = scene.info()
```

## `input.bind(name, binding)`

Bind a named action. Lua definitions override input.toml defaults.

```lua
input.bind("jump", {keyboard={"space"}, gamepad={"a"}})
```

## `input.bindAxis(name, binding)`

Define a digital/analog axis. Digital nonzero input takes precedence.

```lua
input.bindAxis("move_x", {negative={"a"},positive={"d"},gamepad_axis="left_x",dead_zone=0.18})
```

## `input.action(name)`

Held state for an existing action. Unknown names raise an error.

```lua
if input.action("jump") then end
```

## `input.held(name)`

Alias for action; sampled once before game.update.

```lua
if input.held("jump") then end
```

## `input.pressed(name)`

Pressed edge for this sampled frame. Poll in update, not raw key callbacks.

```lua
if input.pressed("jump") then end
```

## `input.released(name)`

Released edge for this sampled frame.

```lua
if input.released("jump") then end
```

## `input.axis(name)`

Named axis in -1..1. Trigger axes are nonnegative.

```lua
local x = input.axis("move_x")
```

## `input.load(path?)`

Read and validate a complete mapping file; replace bindings and clear cached edges.

```lua
input.load("input.toml")
```

## `input.serialize()`

Serialize current bindings to TOML. Does not write project files.

```lua
local text = input.serialize()
```

## `input.apply(toml)`

Validate serialized mappings before replacement; invalid input retains previous bindings.

```lua
input.apply([[
[actions.jump]
keyboard=["space"]
]])
```

## `inspector.expose(path, target, key, metadata?)`

Expose a chosen plain-table field. Read-only unless writable=true; no global enumeration.

```lua
inspector.expose("Player.speed", player, "speed", {writable=true,min=0,max=500})
```

## `inspector.remove(path)`

Remove an exposed field.

```lua
inspector.remove("Player.speed")
```

## `inspector.clear()`

Remove all exposed fields without modifying game data.

```lua
inspector.clear()
```

## `animator.new()`

Create a Lua animation state machine using existing animation clips.

```lua
local controller = animator.new()
```

## `particles.new(options)`

Create a bounded native emitter, optionally with a texture handle.

```lua
local sparks = particles.new({rate=0,lifetime={0.2,0.6},speed={80,220},gravity=200})
```

## `particles.load(path)`

Load native emitter configuration; optional image path is relative to the preset.

```lua
local sparks = particles.load("effects/sparks.particle.toml")
```

## `audio.loadSound(path, options?)`

Alias of audio.load; resource caches decoded data, wrapper stores a default bus.

```lua
local sound = audio.loadSound("assets/hit.wav", {bus="sfx"})
```

## `audio.setBusVolume(bus, volume, fadeSeconds?)`

Set master/music/sfx/ui volume in 0..1 with a fade of 0..30 seconds.

```lua
audio.setBusVolume("music", 0.6, 0.2)
```

## `audio.muteBus(bus, muted)`

Mute without losing the remembered bus volume.

```lua
audio.muteBus("sfx", true)
```

## `audio.stopBus(bus)`

Stop voices on a bus; master stops every voice.

```lua
audio.stopBus("sfx")
```

## `audio.mixer()`

Bus names, target volumes, mute state and voice counts. Not peak metering.

```lua
local buses = audio.mixer()
```

## `debug.drawPhysics(enabled)`

Development-only collider outlines and centers; sleeping bodies have a distinct tint.

```lua
debug.drawPhysics(true)
```

## `debug.drawBounds(enabled)`

Bounded textured-sprite outline overlay, using the queued camera/viewport.

```lua
debug.drawBounds(true)
```

## `debug.drawVelocities(enabled)`

Velocity vectors when physics debug drawing is enabled.

```lua
debug.drawVelocities(true)
```

## `debug.showProfiler(enabled)`

Show/hide the measured CPU/render overlay when profile tools are enabled.

```lua
debug.showProfiler(true)
```

## `replay.bookmark(label, note?)`

Pin a supported snapshot in memory: at most eight pins / 16 MiB. Development tools required.

```lua
local id = replay.bookmark("Wall collision", "Retest from this state")
```

## `replay.getBookmarks()`

Metadata for current runtime pins. Snapshot bytes remain private to the runtime.

```lua
local marks = replay.getBookmarks()
```

## `prefab.spawn(name, overrides?)`

Deep-copy prefabs/name.toml [defaults] and apply overrides; optional explicit Lua factory.

```lua
local coin = prefab.spawn("coin", {x=100,y=200})
```

## `prefab.instantiate(path, parent, overrides?)`

Instantiate a hierarchical JSON prefab under a scene node with fresh IDs and optional root property overrides.

```lua
local enemy = prefab.instantiate("enemy", level.root, {hp=50})
```

## `localization.setLanguage(id)`

Select a game locale from locales/id.json. Does not translate the editor.

```lua
localization.setLanguage("fr")
```

## `localization.setFallback(id)`

Set the fallback locale.

```lua
localization.setFallback("en")
```

## `localization.getLanguage()`

Return the active game-language identifier.

```lua
local language = localization.getLanguage()
```

## `localization.reload()`

Clear the locale cache; subsequent lookups reread project files.

```lua
localization.reload()
```

## `localization.get(key, values?)`

Flat-key lookup with fallback and scalar {name} interpolation; returns missing keys verbatim.

```lua
local text = localization.get("score", {value=10})
```

## `save.registerMigration(from, to, callback)`

Register an adjacent migration. Duplicate/missing/future-version chains raise errors.

```lua
save.registerMigration(1, 2, function(data) data.coins=data.coins or 0; return data end)
```

## `save.writeVersioned(name, version, data)`

Write a versioned JSON envelope in the game save directory.

```lua
save.writeVersioned("progress.json", 2, {coins=10})
```

## `save.readVersioned(name, targetVersion)`

Apply a validated migration chain in memory; does not automatically rewrite the save.

```lua
local progress = save.readVersioned("progress.json", 2)
```

## `filesystem.readJson(path)`

Read project-relative JSON, limited to 1 MiB.

```lua
local data = filesystem.readJson("data/items.json")
```

## `filesystem.readToml(path)`

Read project-relative TOML, limited to 1 MiB.

```lua
local data = filesystem.readToml("prefabs/coin.toml")
```

## `Animator:add(name, clip, options?)`

Register a clip; the first state becomes active. Optional enter/leave callbacks.

```lua
controller:add("idle", idle, {looping=true})
```

## `Animator:set(name, restart?)`

Switch/reset a state. onTransition(previous,next) may be assigned on the machine.

```lua
controller:set("jump", true)
```

## `Animator:transition(from, to, condition)`

Predicate or string "finished". First matching edge wins; one transition per update.

```lua
controller:transition("idle", "run", function() return moving end)
```

## `Animator:update(dt)`

Update current clip and evaluate transitions.

```lua
controller:update(dt)
```

## `Animator:draw(...)`

Draw current clip using graphics.draw arguments.

```lua
controller:draw(player.x, player.y)
```

## `Animator:current()`

Get current animation-state name.

```lua
local state = controller:current()
```

## `Animator:setSpeed(speed)`

Apply speed 0..100 to all clips.

```lua
controller:setSpeed(1.2)
```

## `Animator:pause()`

Pause the active clip; predicates still evaluate if update is called.

```lua
controller:pause()
```

## `Animator:resume()`

Resume the active clip.

```lua
controller:resume()
```

## `Emitter:emit(count)`

Emit up to remaining capacity; return number emitted.

```lua
sparks:emit(30)
```

## `Emitter:update(dt)`

Advance native simulation. dt must be finite and within 0..1 second.

```lua
sparks:update(dt)
```

## `Emitter:draw()`

Append particle quads during draw; alpha blend and square size only.

```lua
sparks:draw()
```

## `Emitter:setPosition(x, y)`

Position future emissions. Existing particles remain in world coordinates.

```lua
sparks:setPosition(player.x, player.y)
```

## `Emitter:clear()`

Clear particles and fractional spawn accumulator.

```lua
sparks:clear()
```

## `Emitter:count()`

Current number of live native particles.

```lua
local count = sparks:count()
```

## `TileMap:update(dt)`

Advance animated tiles, dt in 0..1 seconds.

```lua
map:update(dt)
```

## `TileMap:getProperties()`

Map custom properties as a key/value table.

```lua
local properties = map:getProperties()
```

## `TileMap:getLayerProperties(name)`

Custom properties of an existing tile layer.

```lua
local properties = map:getLayerProperties("ground")
```

## `TileMap:getTileProperties(gid)`

Tileset properties of a global tile ID.

```lua
local properties = map:getTileProperties(1)
```

## `TileMap:getCollisionRects(layer?)`

Centered pixel rectangles with radians rotation; explicit collision=true layers/objects by default.

```lua
local walls = map:getCollisionRects()
```

## `TileMap:findObject(name)`

First object with this name, or nil; coordinates retain Tiled object semantics.

```lua
local door = map:findObject("door")
```

## `TileMap:setLayerOpacity(name, opacity)`

Set one tile-layer opacity override in 0..1.

```lua
map:setLayerOpacity("clouds", 0.5)
```

## `TileMap:reload()`

Load a candidate map, then replace this handle if successful. Does not recreate physics bodies.

```lua
map:reload()
```
