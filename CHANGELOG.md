# Changelog

## Unreleased development work — 2026-09-26

- Added a validated hierarchical scene resource with ordered nodes, stable file IDs,
  stale-handle protection, inherited cached transforms, tags, search and JSON files.
- Added Lua node bindings, per-node script instances and hierarchical JSON prefabs
  with nested-reference cycle detection. Existing scene-stack and TOML prefab APIs remain.
- Added node and global Lua signals with validated node ownership, immediate cleanup
  on subtree destruction, bounded dispatch, and safe listener changes in callbacks.
- Added a focused `.scene` editor tab with tree, node-marker viewport, basic inspector,
  sibling ordering, reparenting by path, bounded tab history and save conflict checks.
  Native node rendering/physics binding
  and full visual scene tooling remain future work.
- Updated the Scene Workshop starter to load an authored scene, attached scripts,
  JSON prefab instances, and a node-owned global event subscription.
- Fixed accepted Kairo Link sockets to use blocking framed I/O on Windows.
- Verified the current checkout with local Windows Cargo gates, example smoke tests,
  and export relocation; see [VALIDATION.md](VALIDATION.md). No new release is declared.

## 3.4.0 - 2026-09-18

This release adds scenes, named input actions, live inspection, particles, audio
buses, tilemap support, replay bookmarks, and project tools. Kairo2D is still
experimental, and this release is published as source code. The native build and
platform checks have not all been completed; see [VALIDATION.md](VALIDATION.md) for
the checks that were run.

### Added
- Queued scene stack/lifecycle/shared state and named animation controllers.
- Serialized input actions/axes, keyboard/mouse/controller edges and mapping editor.
- Explicit typed Live Inspector with read-only defaults, validated opt-in writes,
  fresh session IDs and expected-value conflict handling; local and remote tools.
- Bounded native particles, shared editor preview and `.particle.toml` presets.
- Four audio buses with volume/mute/fades/stop plus playback pitch/pan.
- Physics/bounds/velocity debug geometry and editor toggles.
- Tiled properties, animated tiles, object rectangle collision data and parallax.
- Replay bug pins with notes, session checks, bounded snapshot storage and metadata
  export; safe timeline-budget preflight before bookmark restoration.
- Runtime build profiles, visible selector and recorded standalone-package profile.
- Fuzzy command palette/quick-open, bounded background project search and recent tabs.
- Data prefabs, versioned save migration chains and localization fallback/interpolation.
- Eight new examples/starters including Signal Yard and a one-way platformer.
- Generated API reference from shared JSON metadata and new native/Lua regression tests.

### Changed
- Link protocol 2 rejects old clients. Existing four-tester support gains independent
  labels, auto/manual revisions, typed commands, telemetry/results and health display.
- Save All / Run / Host includes dirty tool documents and preserves conflict checks.
- CLI builds default to runtime Release profile; editor uses its selected profile.
- Development tools are explicitly gated; profile selection does not optimize binaries.

### Deliberate limits
No LAN discovery, CRDT editing, screenshot transfer, universal deterministic rewind,
scene fades, animator graph, streaming audio, asset-import sidecar expansion, dynamic
plugin loader or compiled artifacts. Link/Replay remain experimental and unaudited.

## 3.3.0 - Feature UI source update

- Permanent wrapping feature bar, project overview, runtime status and F6-F10 shortcuts.
- Direct Fantasy Console settings: enable, resolution presets, palettes, Save & Run/Restart.
- Corrected the invalid `olive` palette option to the runtime's `olive4` name.
- Visible project starters, guide/snippet access for game UI and debug UI.
- Link Host/Join flows with explicit trust acceptance, token-file selection and supervised tester launch.
- Replay recording setup and one-action enable/run; useful idle states in Replay/Profiler.
- Shared settings-save guards for dirty config tabs and external edits; demo creation retains unsaved-change prompts.
- Animation picker and re-opening existing animation metadata.
- Dual-binary development launcher and nine additional Rust tests written.

No Cargo build, Clippy, rustfmt, Rust test or GUI/network execution could run locally.
See VALIDATION.md. The patch does not claim production readiness or change the
wire protocol, dependency versions, engine renderer or Lua API.

## 3.3.0 - development source snapshot

Version numbering jumps from 0.2.x by project direction; this is not a claim of
production maturity. Native toolchain unavailable: no Rust build/test/Clippy/fmt,
GUI, GPU, network or release binary result is asserted.

Added in source: font-atlas text, explicit animation clocks and metadata editor,
finite Tiled JSON maps, gilrs gamepads, scoped JSON saves, CPU profiler/control
telemetry, retained Lua UI, pointer-driven egui debug UI, Kairo Micro presentation,
explicit Replay snapshots/RNG/kinematic restoration, experimental encrypted Link
revisions and state hooks, and a statically compiled Rust extension seam.

Improved runtime discovery diagnostics, Windows dual-binary build/packaging workflow,
quick-open/comment/duplicate/function navigation, sprite selection/shapes/crop/pad,
configuration preservation, examples/templates, tests and documentation. Prior
set_enabled/float-literal and boxed enum fixes are retained.

No C#/C++ scripting, WASM loader, full debugger, semantic language server, map editor,
streaming music, shader API or universal rewind is exposed.

## 0.2.0 - 2026-09-16 (unverified development source)

### Added
- `kairo-project`: embedded templates, scoped mutations, atomic saves, TOML-preserving
  settings and host-native directory packaging shared by CLI and editor.
- `kairo-editor`: project hub/recents, resizable workspace, editable highlighted
  Lua/text tabs, histories, search/replace, snippets, console/source links, runtime
  process management, project settings, previews and export.
- Actual RGBA sprite editing with pencil/eraser/fill/picker, zoom/pan/grid, flips,
  undo/redo, conflicts and PNG saving.
- Breakout example/template, host export and additional editor/project/runtime tests.
- Viewport/scissor batch state, per-texture filtering/release/revisioned reload,
  physics contact polling, orderly quit and editor stdin control.
- Windows/Linux/macOS desktop-artifact workflow and release assembler; Windows
  output separates `Kairo.exe` from `bin/kairo.exe`.

### Changed
- Lua replacement is prepared before a successful load swaps the session. Failed
  reload retains the previous VM/resources; module caches refresh on success.
- Config parsing tolerates custom fields; editor writes preserve unrelated settings
  and comments, and detect disk conflicts.
- Documentation distinguishes implemented source, optional omissions and unverified
  build/device behavior. Existing checked non-reused resource IDs are retained.

### Validation status
No Cargo toolchain or working toolchain download was available. No compiler,
Rust test, rustfmt, Clippy, GUI, GPU, audio-output or Windows binary result is
claimed. See VALIDATION.md for actual non-Rust checks. This is not a stable release.

## 0.1.0 - 2026-09-16 (uncompiled source snapshot)

Initial six-crate Rust/Lua runtime with winit/wgpu, texture/sound caches,
input, Rapier wrappers, lifecycle, full-session restart, headless runner,
five examples, documentation and Rust tests written but not executed.
