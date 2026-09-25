# Animation state machines

`animator` is a bundled Lua controller over native animation clips. It does not add a second native resource type.

```lua
local controller = animator.new()
controller:add("idle", idle_clip, {looping=true})
controller:add("run", run_clip, {looping=true})
controller:transition("idle", "run", function() return moving end)
controller:transition("run", "idle", function() return not moving end)
function game.update(dt) controller:update(dt) end
function game.draw() controller:draw(100, 100) end
```

The first added state becomes active. `set(name, restart?)` changes states and restarts the destination clip; setting the same state is a no-op unless restart is true. Options on `add` accept `looping`, `enter(name, previous)` and `leave(previous, next)`. Assign `controller.onTransition(previous,next)` for notifications. `current()` returns the active name.

Transitions are evaluated in registration order after advancing the current clip. The first matching edge wins, with at most one transition per call. A condition is a Lua predicate or `"finished"`; use a nonlooping source clip for completion transitions. At most 256 edges are accepted. `setSpeed` accepts 0..100 and updates every clip.

`pause`/`resume` pause only the active clip. Transition predicates still run if `update` is called while the clip is paused; stop updating the machine to freeze the whole controller. Do not update the same clip separately, or use one mutable clip concurrently in unrelated machines.

The existing animation metadata editor previews individual clips; there is no node graph, state-machine editor, skeletal blending or crossfade. Animation controller state is not automatically in Replay. Signal Yard's `game.replayRestored` resets its transient controller explicitly.
