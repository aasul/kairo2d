# Live Inspector and remote inspection

Open **Live Inspector** from the permanent FEATURES bar or Ctrl/Cmd+Shift+P. Run a game with the Development or Debug profile. Select **Local game** or a connected authenticated Link tester. Remote targets are never silently replaced by the local target when they disconnect.

```lua
local player = {speed=200, health=10, position={80,120}, tint={0.4,0.8,0.7,1}}
inspector.expose("Player.speed", player, "speed", {writable=true,min=0,max=500})
inspector.expose("Player.health", player, "health") -- intentionally read-only
inspector.expose("Player.position", player, "position", {kind="vector",writable=true})
inspector.expose("Player.tint", player, "tint", {kind="color",writable=true})
```

Exposure is explicit: no global walk, Lua evaluation, address inspection or shell. Targets are plain tables. Supported kinds are `number`, `boolean`, `string`, `vector` (2..4-number array), `color` (four channels in 0..1), and bounded `table`. Scalars/tables infer their kind; specify vector/color deliberately. String metadata may include `choices={"idle","run"}`. `writable` defaults false. Functions, threads, userdata, metatables, cycles, aliases and nil values fail.

The editor groups dot-separated paths, provides typed controls, and sends edits only when Apply is clicked. Each request carries a fresh VM session identifier and the expected old value. Rust revalidates permission, type, shape, bounds and current value before applying it. Pause or Reset a draft if another update changed the value. Nested table edits preserve target identity; adding/removing keys or changing value types is not supported. Remove an exposure with `inspector.remove(path)` or clear the registry with `inspector.clear()`.

Limits: 64 exposed fields; 128-byte paths; 2 KiB per field; depth four / 64 value nodes; 512-byte strings; 24 KiB inspection snapshot. Changing a field to an invalid value in game code makes inspection fail visibly rather than serializing arbitrary state.

The same panel displays current/registered scenes and stack, pause/resume/single-step, scene switch/push/pop/reload, supported physics/bounds toggles, game locale preview and bug bookmarks. Use scene commands deliberately: callbacks can reset game state. Locale selection calls the game's localization module, not editor translation.

Local telemetry is sampled at up to 5 Hz; Link exchanges occur roughly twice a second. Unchanged inspector snapshots are omitted from remote transfers. The Link panel reports actual exchange round-trip time, which includes protocol work, not raw ICMP ping. Compare-and-set can reject rapidly changing fields on slow links. Pause first.

`InspectNode`, `InspectMetadata`, `InspectUpdate`, `InspectSnapshot` and `DebugCommand` live in `kairo-core`. The Lua registry and editor are separate consumers. Session IDs change after reload/process restart. The current API is versioned with the engine, not a promised permanent wire ABI. See [Link](kairo-link.md) and [Inspector Demo](../examples/live-inspector/README.md).
