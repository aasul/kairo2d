# Prefabs

Kairo supports hierarchical JSON prefabs as well as the existing TOML data factories. A `.prefab` file uses the same version 1 hierarchy format as a `.scene` file. Its `name` must match the root node's name. You can author it in the editor's text tab or produce the JSON from `graph:toJson()`.

```lua
local level = scene.new("Level")
local enemy = prefab.instantiate("enemy", level.root, {hp = 50})
enemy.position = {x = 400, y = 200}
scene.switch(level)
```

`prefab.instantiate("enemy", parent, overrides)` loads `prefabs/enemy.prefab`; a full `prefabs/...prefab` path is also accepted. Every instance receives fresh scene file IDs and validated runtime handles. The optional table overrides root node properties. Child nodes can reference another prefab through their `prefab` field in the JSON resource. Nested references are expanded before the scene changes; dependency cycles and paths outside the project are rejected. The source prefab path is retained on the instantiated root for inspection. Existing TOML `prefab.spawn` behavior remains available below.

## TOML data factories

```toml
# prefabs/coin.toml
[defaults]
x = 0.0
y = 0.0
value = 1
[defaults.visual]
texture = "assets/coin.png"
```

```lua
local coin = prefab.spawn("coin", {x=100, visual={texture="assets/gold.png"}})
```

Spawn loads project-relative TOML, deep-copies `[defaults]`, and recursively applies overrides without changing another instance. Resource paths remain strings until game code explicitly loads them. Nested tables/arrays are data, not scene nodes. `factory="scripts.coin"` at the TOML root optionally names a normal Lua module returning a function; the function receives merged data and returns the result.

TOML files are limited by the structured-read limit (1 MiB); copies have a 12-level / 4096-node bound. Names cannot escape `prefabs/`. Factory code has ordinary game-code authority; there is no native plugin loading or arbitrary filesystem import. The hierarchical JSON path has no visual prefab editor yet. Signal Yard uses a TOML charge prefab.
