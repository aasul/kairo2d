# Signal Yard

A small top-down collection game integrating scenes, action mappings, Tiled properties/rectangle collisions, Rapier bodies, sprite animation states, native particles, SFX bus, versioned saves, localization, prefabs, explicit inspection, and replay bookmarks. WASD/arrows/left stick move; Enter/A starts; Escape/Start pauses; B pins a bug state. F11 also pins in development. Paused/result/menu scenes suspend physics. Bookmarks require the same scene stack and body topology; restore gameplay pins while in gameplay. Transient particle and animation state is reset explicitly by `game.replayRestored`.

Run with `kairo run examples/top-down` from the repository root, or create the corresponding starter in Kairo.
