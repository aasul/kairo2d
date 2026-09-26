# Architecture

## Boundaries and ownership

Kairo is a workspace, not a wrapper around another game engine. Lua calls validated
functions in `kairo-lua`; those mutate a concrete `EngineState` through a single
thread's `Rc<RefCell<_>>`. Rendering, audio and physics sit behind Rust services.
Lua userdata wrap Kairo handles and simple metadata. No raw wgpu, Kira, Rapier or
eframe object is exposed to a script.

Handles use globally unique, monotonically allocated u64 IDs that are never reused.
This gives stale-handle non-aliasing without recycling slots; it is not a literal
index/generation arena. Allocation exhaustion returns an error. Session and asset
maps own resources; Lua garbage collection alone does not destroy physics bodies.

A `GameSession` owns the VM, resources and lifecycle. Replacement builds a separate
session, optionally encodes `game.saveState`, executes the replacement entry/load
and restore hooks, then swaps. Registered native Rust extensions are shared across
replacement sessions; they are trusted host objects, not hot-loaded libraries.

## Frame path

Input updates polling state before invoking callbacks. Replay requests are applied
at frame boundaries. A normal tick runs retained UI click handling, extension and
Lua update callbacks, deferred scene operations, fixed-step physics (unless the
active scene pauses it), optional replay capture, Lua/scene/native draw,
retained UI draw and debug-UI command collection. Named actions are sampled once before update. Deferred inspector/control operations
are applied at safe boundaries, never by evaluating arbitrary received Lua.
A paused replay still draws and
processes controls but skips gameplay update and physics unless stepping.

Lua graphics calls append commands. The CPU mesh builder preserves painter order
and merges **adjacent** compatible texture/viewport/scissor batches. It does not
globally sort transparent sprites. Bitmap text expands to quad runs; font text uses
rasterized glyph atlas quads. Texture revisions invalidate GPU copies. Uploads are
lazy; frame commands are encoded and submitted together.

Micro draws that same queue into an internal-resolution texture, then presents it
through nearest sampling and optional palette quantization. The egui debug painter
runs afterward at window resolution. There is still one queue submission per
presented frame, including any renderer callback command buffers. Public custom
shaders and render targets are not exported.

## Assets and game systems

`kairo-assets` owns textures and cached font/size pairs. Tilemaps cache parsed maps
per canonical path; their textures use the common texture cache. Animation clocks
are explicit per-instance values. `kairo-core` holds backend-independent frame math,
input models and config, plus the current Rapier wrapper. Adding a crate requires a
real ownership boundary rather than a naming preference.

Project reads are rooted through `ProjectFs`; editor mutation goes through
`ProjectFiles`, which rejects symlinks and uses atomic saves. `SaveStore` uses a
separate user-data root with simple JSON filenames. No OS-level sandbox is claimed;
check/use races and hostile native decoders remain outside this trust model.

## Editor and runtime process

The editor uses eframe 0.28 with its own OpenGL UI backend; games use winit 0.30 and
wgpu 0.20. The runtime egui painter uses a small explicit pointer bridge instead of
mixing incompatible winit versions through egui-winit. It supports pointer-driven
debug widgets, not arbitrary text input or all platform integrations.

The editor reuses project/settings/export code and supervises a separate runtime.
stdin carries bounded local debug commands and stop; EOF stops the runtime. stdout
carries `@kairo:telemetry` JSON at 5 Hz in controlled mode. Other lines feed a bounded
console. Run, Check and Export share the same runtime path resolver. Child cleanup
has a graceful interval and a termination fallback; no shell command is assembled
from project filenames.

## Link and Replay

`kairo-link` is a transport/data crate with no dependency on the VM. Host revisions
are captured, hashed, staged and validated through a supplied project-check callback.
The receiver loads a separate candidate VM from a separate temporary tree. Network
workers have bounded channels, authenticated encryption and size limits. Remote
Lua authority is explicit and powerful; read the threat model before using it.

`kairo-replay` stores bounded encoded snapshots and a deterministic non-cryptographic
RNG. Lua registers plain state through a strict codec. Rapier restoration currently
covers body kinematics/gravity/accumulator with fixed topology, not solver caches.
This is a snapshot debugger, not a universal deterministic rewind engine.

## Extension point

`NativeExtension` and `Canvas` in core expose input and validated queued rectangle/
sprite drawing to statically compiled Rust extensions. The embedding example lives
in `kairo-lua/examples/native_extension.rs`. The API is source-level experimental;
there is no ABI guarantee, dynamic loader, C#/C++ binding generator or WASM runtime.

## Build status

Architecture and code inspection do not replace compilation. The 3.4 authoring
environment had no Rust tools. All native integrations need the acceptance checks
in [release-acceptance.md](release-acceptance.md).

## 3.4 additions

Scenes and animation-state orchestration are bundled, bounded Lua modules, not
Rapier/wgpu objects or a new ECS. Particles and action state are concrete Rust models
with device-independent tests. Lua wraps emitters; per-particle objects are not Lua
allocations. The editor previews the same emitter implementation.

Inspector data/validation, DebugCommand and Telemetry live in core. The VM owns
explicit table registrations and validates fresh session, expected old value, shape,
range and write permission before editing. Local stdin and Link carry the same
structured controls. Link protocol 2 rejects the 3.3 wire protocol; up to four tester
workers retain bounded commands/results and telemetry independently.

Bug bookmarks pin bounded encoded snapshots separate from rolling history. Preparing
a replacement timeline validates its memory budget before physics/time mutation.
Physics restoration still requires fixed topology and does not restore solver caches.
Profile settings are resolved once per launch/candidate. Project export records the
profile; it copies a built runtime rather than compiling optimization variants.

Shared project services own templates, settings, mutation guards, fuzzy matching and
bounded search. Editor tool saves use the same dirty-tab/external-change checks as
other project writes. API JSON metadata drives snippets and generated reference docs;
`generate_api_docs.py --check` prevents reference drift.
