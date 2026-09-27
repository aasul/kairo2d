# Scenes and state stacks

Scenes keep their existing Lua state-stack callbacks and now also own a validated Rust node hierarchy. The engine installs `scene` before `main.lua`.

```lua
local play = scene.new("play")
function play:enter() self.elapsed = 0 end
function play:update(dt) self.elapsed = self.elapsed + dt end
function play:draw() graphics.print("Playing", 30, 30) end
scene.register(play)
function game.load() scene.switch("play") end
```

`scene.new(name)` creates an unregistered table. Names contain 1..64 bytes; duplicates fail. `register` validates callback types, with a maximum of 64 scenes. The stack contains at most 16 distinct scenes. `scene.shared` is an ordinary Lua table for state that survives switches, not a save file or automatically replayed state.

## Node hierarchy

`scene.new` gives each scene a `graph` and `root`. Existing scene callbacks continue to work. A scene can be switched by name or by its table (which registers it on first use):

```lua
local level = scene.new("Level")
local world = level:createNode("Node2D", "World")
local player = world:createChild("Sprite", "Player")
world.position = {x = 100, y = 80}
player.position = {x = 16, y = 0}
assert(player.globalPosition.x == 116)
player:addTag("hero")
scene.switch(level)
```

Nodes have names, types, file IDs, ordered children, local position/rotation/scale, inherited `globalPosition`, enabled and visible flags, tags, properties, and an optional script path. Use `children()`, `parent()`, `findChild(name)`, `path()`, `reparent(parent, index)`, `reorder(index)`, `duplicate()`, and `destroy()`. Sibling indexes in Lua start at 1. `graph:findPath(path)` and `graph:findByTag(tag)` search the hierarchy. Destroyed node references return errors; they never refer to a new node. Parent cycles and non-finite transforms are rejected.

`graph:toJson()` produces a versioned, human-readable `.scene` document. Save that text as a project file and load it with `scene.load("scenes/level.scene")`. The editor opens `.scene` files in a visual resource tab with a scene tree, 2D node markers, pan/zoom grid, and an inspector for name, local transform, visibility, script path, and typed native properties. An empty `.scene` file opens as a new scene named from its filename. Save writes the same resource loaded by Lua. The scene tab can reorder siblings and reparent by path, has bounded Undo/Redo for its edit commands, and refuses to overwrite an externally changed file. Node markers can be dragged with Snap 16. Tree drag-to-reparent and rotation/scale gizmos are not yet available.

Scene files carry the ordered hierarchy, file IDs, transforms, tags, script paths, metadata, and node properties. The hierarchy is currently limited to 50,000 nodes and 256 levels. The active scene now turns supported node properties into native draw commands, Rapier bodies and audio playback. Call `node:setProperty(name, value)` from Lua to change an authored property at runtime; `node:removeProperty(name)` removes it.

The [live inspector](live-inspector.md) can page through the active runtime
hierarchy and edit explicitly exposed node fields. A field marked
`persist=true` can be applied back to its authored `.scene` file after the
editor verifies the source node and prior value. The [resource analyzer](resource-dependencies.md)
reports known scene and prefab dependencies, broken references, and potential
unused assets.

## Native scene properties

The editor's inspector provides typed controls for the supported properties below. Other JSON properties remain available for scripts and prefabs. Nodes draw in hierarchy order after the scene's `draw` callback and before attached node `draw` callbacks. `graphics.clear` discards earlier commands, so put a scene background clear in the scene's `draw` callback.

| Node | Properties | Runtime behavior |
| --- | --- | --- |
| `Sprite`, `AnimatedSprite` | `texture` project path, optional `source` object with `x`, `y`, `width`, `height`; optional `width`, `height`, `tint` RGBA array, `flip_x`, `flip_y` | Cached texture and exact inherited affine transform enter the ordered native draw queue. `AnimatedSprite` can change `source` from Lua; clip playback is still the separate animation API. |
| `Camera2D` | `active` boolean, `zoom` positive number | The first enabled active camera in tree order centers on its node world position and inherits rotation. `graphics.worldToScreen` and `graphics.screenToWorld` use it after scene drawing. |
| `CanvasLayer` | none | Descendant draw nodes use screen coordinates, independent of the active camera. |
| `Control` | `width`, `height`, `color` RGBA array, `interactive` boolean | Draws a transformed solid rectangle. Interactive controls receive a `click` node signal and attached script `onClick(self, x, y)` after a press/release inside the same control. Topmost drawn control wins. |
| `Text` | `text`, `scale`, `color` RGBA array | Draws bitmap text at the inherited position. Text glyphs do not yet rotate or shear with parents. |
| `StaticBody2D`, `DynamicBody2D`, `CharacterBody2D`, `PhysicsBody2D` | One direct enabled `Collider2D` child; optional `velocity` as `{x,y}` or `[x,y]`, `gravity_scale`; `PhysicsBody2D` can set `body_type` to `static` or `dynamic` | Uses the existing Rapier world. Character bodies default to zero gravity. Dynamic poses and velocity write back to the graph after each physics step; changing `position`, `rotation` or `velocity` from Lua updates the native body on the next step. |
| `Collider2D` | `shape` of `rectangle` or `circle`; `width`/`height` or `radius` | Shapes are centered on the body origin and inherit its world scale. A collider must have the default local transform. Invalid or missing collider data reports scene and node path. |
| `AudioSource` | `sound` project `.wav`/`.ogg` path, `autoplay`, `looping`, `volume` in 0..1, `bus` | Loads through the existing audio manager. An active source starts playback when a device exists and stops on disable, destruction or scene deactivation. With `--no-audio`, the asset still loads but no voice plays. |

An authored body currently supports one direct rectangle or circle collider. Scene-owned native body contacts emit a `collision` signal and call attached `onCollision(self, other)` once per frame using the latest fixed-step contact state. Collider offsets, capsules, sensors, `Area2D`, trigger enter/exit, contact normals, contacts with manually created physics bodies, and physics hierarchy shear are not supported by this bridge. AudioSource is non-positional. Scene `Control` supports pointer clicks; keyboard/controller focus, text entry, scrolling, and layouts still use the existing `ui` API or game Lua. The Scene Workshop starter contains a playable native body, sprite, following camera, HUD, and prefab colliders.

## Attached Lua scripts

```lua
-- scripts/player.lua
local Player = {}
function Player.ready(self) self.health = 100 end
function Player.update(self, dt) self.health = self.health + dt end
return Player
```

```lua
local state = level:attachScript(player, "scripts/player.lua")
```

Each attached node gets a separate `state` table containing its `node` reference. The returned module table can provide `ready`, `update`, `fixedUpdate`, `draw`, `onDestroy`, `onCollision`, `onTriggerEnter`, and `onTriggerExit`; Kairo registers only callbacks that exist. Saved script paths are loaded when a scene is registered. `ready`, `update`, and `draw` are dispatched by the current scene lifecycle. Scene-owned native body contacts invoke `onCollision(self, other)` once per frame based on the latest fixed-step contact state. Trigger enter/exit callbacks and `fixedUpdate` are not yet wired to a native scheduler. Errors include scene, node path, script, callback, and the Lua error message.

## Signals

```lua
local token = player:on("health_changed", function(health)
    healthBar:setProperty("health", health)
end)
player:emit("health_changed", 75)

local roundToken = Events.on("round_end", function(score)
    print(score)
end, player) -- optional owner node
Events.emit("round_end", 100)
Events.off(roundToken)
```

`node:on` listens only to that node's events. `Events.on` listens to global events and can take an owner node. The registry is local to each Lua game VM. Subscriptions owned by a destroyed node, and subscriptions to that node's events, disconnect when its subtree is destroyed. `node:off(token)` or `Events.off(token)` disconnects early. Dispatch is synchronous; adding or removing listeners from a callback is safe. Newly added listeners run on the next emission. Events have a 64-byte name limit, 4096 live-subscription limit, and 32-level recursive dispatch limit. A callback error stops that emission and reports the event name.

`switch(name)` leaves the entire old stack and enters the new scene. `push(name)` calls the previous top's `suspend`, then the new top's `enter`. `pop()` calls `leave` and the exposed scene's `resume`; popping the only scene is an error. These transitions release the old active scene's native physics bodies and audio voices. `reload()` means leave/re-enter, not source hot reload. `current()` returns a name or nil. `info()` returns registered names, stack and current name.

The original game callbacks remain active. The engine invokes `game.update`, then the top scene's `update`; do not put pausable gameplay in both. Input callbacks go to the top. Draw traverses bottom to top, starting at the highest `opaque=true` scene. Only `update`/draw/input callbacks are automatically delegated; `game.debugUI`, `game.load`, `game.quit` and game state-transfer hooks remain global.

Changes made inside callbacks are queued and flushed between non-draw callbacks. Draw-time changes wait for the next boundary. A callback chain can process at most 32 queued changes; invalid/recursive chains report Lua errors and clear the pending queue. Callbacks can mutate data before an error: this is not a transactional scene rollback.

Set a pause overlay's `pause_physics=true` to suspend the global physics step. Its default is false. Underlying scenes stop receiving updates regardless. Drawing state is shared, so a screen-space overlay should reset the camera/color explicitly. `graphics.clear` still discards prior queued geometry.

No fades, crossfades or automatic resource unloading are implemented. Scenes remain registered until VM reload. Replay restoration requires the same stack; it does not run scene transitions to reconstruct another scene. See [Scene Demo](../examples/scenes/README.md) and [Signal Yard](../examples/top-down/README.md).
