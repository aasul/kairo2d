# Replay and bug bookmarks (experimental)

Kairo captures explicit supported state, not arbitrary process memory. Register plain Lua tables with `replay.register(name, table)`. Configured recording captures bounded rolling snapshots at a chosen frequency, default 30 Hz / 10 seconds / 32 MiB. Native snapshots include frame/time, Kairo RNG, polled input/action bindings/states, and supported physics-body kinematics. Lua functions, userdata, threads, metatables, cycles and aliased tables are rejected.

```lua
local player = {x=50, health=10}
replay.register("player", player)
replay.enable(true)
-- From an event during play:
local bookmark_id = replay.bookmark("Wall collision", "Check the contact")
```

The Replay panel provides record, pause, resume, single-step, timeline seek and bookmarks. Lua also has capture/stats/clear/rewind/seek/pause/resume/step. A paused session still draws. Resuming branches away from future rolling snapshots and clears held input. Snapshots preserve registered root/nested table identity on restoration where supported. `game.replayRestored()` is called afterward; use it to rebuild/reset transient effects.

## Bug bookmarks

Press **F11** in the editor or running development game, press **B** in the supplied bug demo, or use Capture in Replay/Live Inspector. A pin stores its own snapshot, so rolling-history eviction does not immediately invalidate it. Metadata includes label/note, scene, frame, simulation time, snapshot byte count and the latest measured profile sample. Up to eight pins / 16 MiB total are retained; oldest pins are evicted to fit. A single oversized pin fails rather than clearing everything.

Edit labels/notes, restore, delete or export metadata from the panel. Restoring a pin replaces the rolling timeline with that point and pauses. Pins are in-memory only and are cleared on VM reload or replay root-registry changes. Exports are explicitly marked `snapshot_included=false`: no screenshot, portable state, call stack or log bundle is claimed. Remote bookmarks remain on the selected tester.

Restoration requires the same scene stack and compatible physics-body topology. It does not automatically switch scenes, recreate destroyed objects, reconstruct solver contact caches or replay external I/O. Audio voices stop; native particles, animation controllers, retained UI state, network messages and files are not rewound. Register gameplay state and use the restoration hook for transient systems. Cross-platform frame-perfect determinism and exact arbitrary bug reproduction are not promised.

Runtime controls and bookmark capture require Development tools. Release profiles disable automatic recording and tools by default. A shipped game may deliberately use the game-facing replay APIs for its own mechanic, but must manage its state contract. See [Replay Bug Demo](../examples/replay-bugs/README.md).
