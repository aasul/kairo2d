# Data prefabs

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

Files are limited by the structured-read limit (1 MiB); copies have a 12-level / 4096-node bound. Names cannot escape `prefabs/`. Factory code has ordinary game-code authority; there is no native plugin loading or arbitrary filesystem import. No inheritance hierarchy, ECS or visual prefab editor is included. Signal Yard uses a charge prefab.
