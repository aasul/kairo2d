# Inspecting a running scene

Run a project with development tools enabled and open **Live Inspector** in
Kairo Editor. The runtime hierarchy shows the active scene graph in pages of
32 nodes. Indentation shows parent and child relationships. Select a node to
see its path, type, ID, enabled and visible state, local transform, tags,
script/prefab path, and a bounded set of stored properties. Selection uses a
runtime session, graph identity, and non-reused node file ID. A destroyed node
or replaced scene is shown as stale; select another node to continue.

Node fields are read-only by default. A game can explicitly expose a field:

```lua
boss:setProperty("attack_delay", 1.25)
boss:exposeInspector("property.attack_delay",
    {kind="number", writable=true, persist=true, min=0.1, max=5})
boss:exposeInspector("property.health", {kind="number"})
boss:exposeInspector("position",
    {kind="vector", writable=true, min=0, max=960})
```

Supported engine fields are `position`, `rotation`, `scale`, `pivot`,
`enabled`, and `visible`. `property.name` addresses an existing stored node
property. Metadata uses the same kinds, ranges, and choices as
`inspector.expose`. The editor sends an expected value with each edit. The
runtime rejects edits when the value has changed, the node is stale, the
field is read-only, or the proposed type/range is invalid. Runtime edits take
effect immediately and do not write project files by themselves.

For a node loaded from an authored `.scene` file, the Inspector compares
runtime and scene values. **Revert Runtime** sends the authored value through
the same validated edit path. **Apply to Scene** is available only when the
field is explicitly marked `persist=true`. The editor verifies the source
scene, node ID, path, type, and prior scene value before writing. It rejects
dirty scene tabs and changes made on disk since the comparison. Applying a
value updates that scene file and reloads a clean scene tab. Use `persist`
only for authored tuning values such as attack delay; transient health and
timers should remain runtime-only. **Apply to Scene** is restricted to the
local runtime; Link peers can inspect and edit live values but cannot write
the host project from their telemetry.

The existing `inspector.expose` API remains useful for plain Lua gameplay
tables. It can expose fields such as health or damage that are not stored on
a scene node. The runtime hierarchy currently shows the top scene graph;
the scene stack is displayed separately. Click selection in the game viewport
and automatic matching of scene-editor nodes are not implemented yet.
Generated scenes have no authored source for Apply/Revert, and nested prefab
resources cannot be applied through this scene-file path. Large property
values and excess fields are omitted from telemetry to keep local and Kairo
Link messages bounded.

Try [examples/live-inspector](../examples/live-inspector/README.md) for a
small balancing workflow.
