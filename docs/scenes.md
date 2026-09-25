# Scenes and state stacks

Scenes are small Lua tables, not an ECS or a visual scene graph. The engine installs `scene` before `main.lua`.

```lua
local play = scene.new("play")
function play:enter() self.elapsed = 0 end
function play:update(dt) self.elapsed = self.elapsed + dt end
function play:draw() graphics.print("Playing", 30, 30) end
scene.register(play)
function game.load() scene.switch("play") end
```

`scene.new(name)` creates an unregistered table. Names contain 1..64 bytes; duplicates fail. `register` validates callback types, with a maximum of 64 scenes. The stack contains at most 16 distinct scenes. `scene.shared` is an ordinary Lua table for state that survives switches, not a save file or automatically replayed state.

`switch(name)` leaves the entire old stack and enters the new scene. `push(name)` calls the previous top's `suspend`, then the new top's `enter`. `pop()` calls `leave` and the exposed scene's `resume`; popping the only scene is an error. `reload()` means leave/re-enter, not source hot reload. `current()` returns a name or nil. `info()` returns registered names, stack and current name.

The original game callbacks remain active. The engine invokes `game.update`, then the top scene's `update`; do not put pausable gameplay in both. Input callbacks go to the top. Draw traverses bottom to top, starting at the highest `opaque=true` scene. Only `update`/draw/input callbacks are automatically delegated; `game.debugUI`, `game.load`, `game.quit` and game state-transfer hooks remain global.

Changes made inside callbacks are queued and flushed between non-draw callbacks. Draw-time changes wait for the next boundary. A callback chain can process at most 32 queued changes; invalid/recursive chains report Lua errors and clear the pending queue. Callbacks can mutate data before an error: this is not a transactional scene rollback.

Set a pause overlay's `pause_physics=true` to suspend the global physics step. Its default is false. Underlying scenes stop receiving updates regardless. Drawing state is shared, so a screen-space overlay should reset the camera/color explicitly. `graphics.clear` still discards prior queued geometry.

No fades, crossfades or automatic resource unloading are implemented. Scenes remain registered until VM reload. Replay restoration requires the same stack; it does not run scene transitions to reconstruct another scene. See [Scene Demo](../examples/scenes/README.md) and [Signal Yard](../examples/top-down/README.md).
