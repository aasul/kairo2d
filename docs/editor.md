# Kairo desktop editor

3.4 retains the permanent 3.3 feature bar and adds visible workflow tools. Source
changes require rebuilding **both** editor and runtime. Launch an old executable
and you will still see its old interface. The updated title says **Workflow Tools**.
Native GUI behavior remains unverified in the authoring environment.

## Start and files

The hub shows sixteen starters, project title/location, Open and recent projects.
A new project must use a new child directory of an existing parent. Removing a
recent entry does not delete its files. The editor never migrates an old game's
code implicitly when a new engine version is installed.

The project browser opens supported code/config/images/audio and animation files.
Creation, rename, duplication and deletion are project-scoped. Delete requires
confirmation and is permanent. Code tabs support syntax coloring, line numbers,
undo/redo, find/replace, goto-line, indentation, comments, duplicate lines and a
function outline. Dirty tabs and tool documents participate in Save All and the
project-close/quit prompt. Conflicting disk edits are rejected, not silently lost.

Recent files and up to sixteen open text/asset tabs per project are restored when
possible. Unsaved contents are **not** crash-recovery backups. Panel dimensions and
preferences persist in the user's configuration directory. This is a resizable
workspace, not arbitrary dockable panes or a visual ECS scene editor.

## Main toolbar and feature bar

Choose **Development**, **Debug**, **Release** or **Micro** beside Run/Stop/Build.
Changing selection affects the next run/export, not an already-running child.
Both actions use the selected profile; choose Release before shipping a game.
This selector does not rebuild or optimize Rust binaries. See [profiles](build-profiles.md).

The wrapping feature bar includes Overview, Fantasy Console, Link, Replay, Profiler,
Lua UI, Inspector, Input Mappings, Particles, Mixer and Search. Command palette gives
keyboard access to these and existing actions without adding more tiny buttons.

| Shortcut | Action |
| --- | --- |
| F5 / Shift+F5 | Run / stop project. |
| F6 / F7 / F8 | Overview / Fantasy Console / Kairo Link. |
| F9 / F10 | Replay / Profiler. |
| F11 | Create a local runtime bug bookmark. |
| Ctrl/Cmd+S / Ctrl/Cmd+Shift+S | Save active / Save All. |
| Ctrl/Cmd+P | Fuzzy quick-open, preferring recent files on similar scores. |
| Ctrl/Cmd+Shift+P | Command palette. |
| Ctrl/Cmd+Shift+F | Project-wide text search. |
| Ctrl/Cmd+F / Ctrl/Cmd+H | Find / replace in the code document. |
| Ctrl/Cmd+/ / Ctrl/Cmd+D | Toggle selected-line comments / duplicate lines. |

The status bar reports selected profile, actual child-process state, available
FPS/current scene, connected Link client count and unsaved status. It does not
invent telemetry when no runtime is connected.

## Live Inspector and scenes

Run a project in a profile with tools enabled, then open **Inspector**. Choose the
local game or one authenticated Link tester. Only explicit `inspector.expose`
registrations appear. Fields default to read-only; writable ones are edited in a
draft and sent with **Apply**. A changed expected value or stale session rejects
that edit. Reset loads the latest snapshot. Vector/color/table values are bounded
and validated. See [Live Inspector](live-inspector.md).

The same panel shows scene stack/registered names, switch/reload/push/pop controls,
physics/bounds/velocity toggles, profiler/replay state and a localization preview
selector. Scene commands are deferred; an acknowledgement is acceptance, not proof
that a later Lua lifecycle callback succeeded. Runtime errors appear in logs.

## Input, particles and mixer

**Input Mappings** edits the shared `input.toml` model: action buttons, axes, player
and dead zone. Save writes validated TOML. Existing unrelated edits are not ignored;
reopen or reconcile an external change. **Run/Restart** loads a changed TOML file.
Raw keyboard/gamepad APIs remain available. See [input actions](input-actions.md).

**Particles** loads/saves `.particle.toml`, edits parameters and previews the actual
Rust emitter model, optionally with a texture. Click/drag in the preview to position
the emitter. Save All includes dirty particle/input documents. These tools retain
their documents when hidden. See [particles](particles.md).

**Mixer** selects the same local/remote targets as Inspector. It exposes four real
buses, target gain, mute, stop and voice counts. Displayed gain is not a measured
VU meter. The audio device must exist for audible output. See [audio mixer](audio-mixer.md).

## Search, commands and documentation

Project Search runs off the UI thread and searches saved UTF-8 Lua, TOML, JSON,
Markdown, text and WGSL files. It is literal search with optional case matching,
limited to 1000 results / 64 MiB scanned and 1 MiB per file. It ignores target, .git,
dist, exports, hidden entries and known credential patterns. Results include relative
path/line and open the corresponding document. It does not search unsaved buffers,
support regular expressions or perform project-wide replacement.

Command palette uses fuzzy subsequence matching for actual actions. Offline API
snippets/search come from `docs/lua-api.json`, also used to generate the bundled
[API reference](api-reference.md). This is not a Lua language server, semantic
completion engine or breakpoint debugger.

## Existing creative/development tools

Fantasy Console: enable the profile, choose canvas/palette/scaling, Save & Run.
A running window must restart for presentation changes. Low resolution does not
rewrite gameplay layouts; use the Micro starter.

Link: Host or Join as tester requires explicit trust and a token file outside the
project. Up to four testers, labels, telemetry, last patch/error and automatic or
manual revision delivery are exposed. Choose a target for live edits. No network
services start from a project file alone. See [Link](kairo-link.md).

Replay: enable recording, pause, step, restore/scrub, resume. Bookmarks preserve
bounded snapshots beyond rolling-history eviction; rename/note/restore/delete and
metadata export are available. Metadata export does not contain a portable replay
snapshot. See [Replay](kairo-replay.md).

Sprite tabs edit actual pixels (pencil/eraser/fill/line/rectangle/selection/copy/paste,
flip, crop/pad, zoom, grid and PNG save). Clipboard is tab-local. The animation tool
edits sprite-sheet `.anim.json` frame timing/preview, not animator transition graphs.
There is no tilemap painter or expanded asset-import sidecar UI in this release.

## Run, errors and export

The editor supervises a separate runtime and streams stdout/stderr into a bounded
console. Recognized local source locations open files. Remote navigation is
best-effort against the local counterpart. Stop requests shutdown, then falls back
to termination; the process is not intentionally orphaned when the editor closes.

Export uses the same project service as the CLI and copies a native runtime plus
project directory. Existing output directories are rejected. Select a release-built
runtime as well as a Release runtime profile for distributions. See
[building games](building-games.md) and [release acceptance](release-acceptance.md).
