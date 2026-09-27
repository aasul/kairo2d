# Resource dependency analysis

Open **Development tools → Resources** and select **Analyze project**. The
panel scans saved `.scene`, `.prefab`, `.anim.json`, `.tmj`, and `.tsj` files for
known resource fields. Enter a project-relative path to see where it is used
and what that resource depends on. Broken references show the referencing
file, target, and JSON field path. The analysis handles cycles when following
transitive dependencies.

The panel labels files with known references as **referenced**, resources
mentioned in Lua source as **dynamic / uncertain**, and remaining assets as
**apparently unreferenced**. These are static hints, not proof of safe
deletion. Lua can construct paths at runtime, and custom JSON fields may not
be recognized. No resource is deleted or excluded from export by this tool.

Project browser rename previews known references. For file moves, it updates
project-relative references in `.scene` and `.prefab` JSON and reloads clean
editor tabs. The operation prepares all rewritten documents before moving
the file and attempts to roll back if a write fails. It refuses a Lua source
mention, relative tilemap/animation references, a dirty referencing tab, or
an uncertain folder move. Recheck the references after an interrupted disk
operation; multi-file writes cannot be fully atomic. Development hot reload
watches scene, prefab, shader, audio, tilemap and animation metadata changes.
It uses the graph to check changed resources for broken references before
replacing the session; on failure, it keeps the prior runtime. A successful
change still reloads the session rather than selectively mutating prefab
instances. Texture changes continue through the existing cached texture
reload path. Packaging still includes all project resources because dynamic
Lua paths cannot be proven unused.
