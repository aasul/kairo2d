# Hot reload

Local development uses notify with a 4096-event bounded queue, 200 ms debounce and
texture-path deduplication. A queue overflow schedules a full session replacement.
Lua/module, JSON/animation, TMJ/TSJ and font changes trigger a candidate VM. PNG/JPEG
changes reload decoded pixels and update revisioned GPU textures only when dimensions
match. Resized textures require F5 or Stop/Run to make a new session.

The running session remains until a replacement entrypoint and `game.load` complete.
Optional `game.saveState()` data is encoded by the strict Replay codec and passed to
`game.restoreState(data)` in the new VM. If the old game provides saveState, the
candidate must provide restoreState. Encoding is capped at 8 MiB of JSON plus the
codec's node/string/depth constraints. New resources have fresh non-aliasing handles.

```lua
local state = {score=0}
function game.saveState() return state end
function game.restoreState(previous) state = previous end
```

Without hooks, state resets. `package.loaded`, physics, asset caches, animations,
scene stacks, action bindings, inspector registrations, particles, mixer settings,
UI objects, bookmarks and Replay history belong to a VM/session and are not automatically
transferred. Native Rust extensions attached by an embedding host remain shared.
Old game.saveState should be side-effect-free; its mutations cannot be rolled back.

Save writes/removals and audio.play are blocked during initial/candidate load and
restore hooks. Load assets there, but start playback during update/input. Window
commands stay in the candidate queue until it becomes active. Other external
side effects are not an all-purpose transaction system.

Validation covers entrypoint, load and explicit restore, not all future code paths.
A syntax/load error retains the previous session and shows a warning. An error in a
later update/draw stops advancing that session and shows an error screen until fixed.
F5 requests a full local replacement. With --no-watch, errors terminate instead.

Local TOML/audio/shader edits are not watched for automatic configuration changes;
Stop/Run after changing those. Link uses the same VM replacement boundary with
separate immutable revision directories, authenticated transport and additional
configuration checks. See [kairo-link.md](kairo-link.md).

3.4 specifics: input.toml, particle/prefab TOML and kairo.toml edits require restart.
Tiled JSON/TMJ/TSJ and JSON localization edits trigger a whole candidate session.
No individual map objects/colliders are magically patched into a running Lua world.
Inspector session IDs change on successful VM replacement; stale edits and bookmark
controls then fail rather than touching a new session. Re-expose/register the
replacement tables during load/restore. Runtime-only edits are not written to source.
