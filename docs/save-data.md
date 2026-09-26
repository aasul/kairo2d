# Save storage and migrations

Project assets and writable saves are distinct. Saves live in the platform app-data location selected by `directories`, under the project's `[save].identity`. New projects receive a random stable identity. Keep it across game updates. Do not put save files or personal paths in the repository.

Base APIs are `save.write(name, value)`, `save.read(name)`, `save.exists(name)`, `save.remove(name)`. They use project-specific storage, validated relative paths and atomic replacement. No arbitrary user-directory file API is exposed. Missing reads return nil. JSON-compatible values only; see the Lua API reference for base limits.

Versioned helpers store an envelope containing `kairo_save_version` and a `data` table:

```lua
save.registerMigration(1, 2, function(data)
    data.coins = data.coins or 0
    return data
end)
local progress = save.readVersioned("progress.json", 2)
if not progress then progress = {coins=0} end
-- Save only when the game deliberately commits progress:
save.writeVersioned("progress.json", 2, progress)
```

Versions are integers 1..10000. Migrations must connect adjacent versions and return a table. Registering the same source version twice fails. A newer-version save, missing migration, malformed envelope or failed callback produces an error; data is not silently discarded. Reads migrate in memory and never overwrite disk. Raw old saves without an envelope require explicit application conversion, not automatic guessing.

As with other writes, startup/candidate callbacks cannot write save data. Read during load if needed; save from accepted game/update actions. Signal Yard demonstrates retaining an incompatible save and reporting the error without replacing it.
