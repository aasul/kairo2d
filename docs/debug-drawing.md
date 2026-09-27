# Debug geometry

`debug` is a small Kairo table, not Lua's unrestricted debug library. It exposes `showProfiler`, `drawPhysics`, `drawVelocities`, `drawBounds`, `drawOrigins`, `drawNodeNames`, and `drawCamera`, each taking a boolean. The Live Inspector also exposes these controls.

Physics drawing shows box/circle collider outlines, body centers, sleeping-body tint and optional velocity vectors (one tenth of a second of velocity). It visits at most 256 bodies; circles use 24 segments. It uses the final drawing camera/viewport, so leave the desired world camera selected after your screen-space HUD. Signal Yard demonstrates this convention.

Sprite bounds capture each queued textured quad's camera, transform, viewport and scissor. At most 1024 textured quads are outlined; the rest are omitted. The implementation gathers only bounded overlay geometry rather than cloning the entire frame. Debug lines join the normal command queue and do not submit GPU work independently.

Node origins and names are drawn from the active authored scene graph, with a limit of 256 visible nodes per scene. Camera drawing outlines the current game view and marks its center. These are development-tool overlays and are omitted when tools are disabled. They do not label legacy immediate-mode draw calls.

Contact points/normals and raycast visualization are not implemented. Collision tiles can be turned into physics rectangles and shown through the physics overlay; there is no separate tile-collision painter.

Release **project profiles** disable runtime tools by default. Debug source is still compiled into the runtime; this is a runtime feature gate, not a guarantee of binary stripping. An explicit profile override may re-enable tools for a trusted development export.
