# Known limitations and native release gates

## Source status

3.4.0 is expanded, uncompiled development source. Cargo/rustc/rustfmt/PowerShell were
unavailable in the authoring environment; installation attempts failed. No Rust
compilation, tests, Clippy/format result, GUI/GPU/audio/controller run, authenticated
network session or native package was produced here. The version number is not a
maturity guarantee. See the exact results in ../VALIDATION.md.

## Implemented but experimental

Link protocol 2 provides opt-in encrypted saved-project distribution and typed
controls for up to four testers, not CRDT text editing, multiplayer game netcode,
relay-backed matchmaking or an audited public service. Reconnect/authentication/
shutdown/multi-client tests exist but did not run. Native DNS can outlast a socket
timeout. Portable ASCII project paths and bounded transfers are required. Whole
revisions can hitch while loading. Token possession grants trusted development
participation; no per-person role system exists. Labels are local session labels.
There is no LAN discovery, QR join, screenshots, file-diff transfer or automatic NAT traversal.

Inspector is explicit, bounded and opt-in. It is not arbitrary Lua evaluation or a
complete object graph. Writes need writable metadata, matching session/expected old
value and validation; gameplay can legitimately invalidate a draft before Apply.
Remote control acknowledgements do not guarantee later scene callbacks succeed.

Replay/bookmarks capture registered plain tables, controlled RNG, polled input/action
state and fixed-topology physics kinematics. Scene stack must match on restore. No
arbitrary memory, contact/solver caches, audio playheads, GPU state, unregistered
locals, scene closures, particles or external file effects are restored. Pins are
session-local, bounded to eight/16 MiB and lost on VM reload. Metadata export is not
portable state import. No guaranteed deterministic rewind or exact forward replay.

Rust extensions are statically compiled source-level host objects. No ABI-stable
plugin loader, WASM runtime or C#/C++ gameplay binding exists.

## 3.4 boundaries

Scenes use immediate queued state changes with callbacks, not visual fade/crossfade
transitions. Scene callbacks are not rolled back after arbitrary side effects. Global
game.update still runs when a pause scene is on top; the application must put paused
logic in the correct scope. Animation states support predicates/completion and
callbacks, but no blend durations, graph UI or semantic animation events.

Input mappings support action/axis remapping, not rebinding by capturing the next
physical button in a modal or complete joystick calibration. Backend radial stick
dead zones precede action-specific axis dead zones.

Particle simulation is native but bounded: 64 emitters and 8192 particles per emitter.
Alpha quads only, square size, explicit update, no scene collisions, trails, lights
or additive pipeline. Seed state is not registered with Replay automatically.
Audio has four fixed buses, not arbitrary routing graphs/effects or streamed music.
Gain/voice count telemetry is not a measured audio level meter.

Tiled support is finite orthogonal JSON with array tile data, simple tile/object
layers, animated tiles, properties and parallax. No LDtk/TMX, infinite maps, groups,
compressed/base64 tile data, image-collection tilesets or diagonal flips. Axis-aligned
rectangle object collision data can be converted into static bodies explicitly;
there is no automatic world collision rebuild on every file change. Tilemap files
reload as a new VM, not in-place object mutation.

Physics debug drawing includes supported body shapes/centers, sleeping colors and
optional velocity vectors. Quad bounds retain their viewport/scissor. There are no
contact points/normals, raycast overlays or independent camera-bounds tool. Physics
lines use the final drawing camera/viewport. Developer drawing is gated at runtime,
not stripped from the Rust binary by the project Release profile.

Runtime profiles control supported configuration, not compiler optimization,
compression or cross-compilation. Some tool APIs remain callable by trusted game code
for intentional gameplay use (for example replay.enable). Release is not a security sandbox.

## Existing subsystem limits

Font rendering uses fontdue and cached atlases with simple advances/kerning, not
complex shaping, bidi, fallback, color emoji or rich layout. No font files are bundled.
Micro has predefined palettes but no custom palette file workflow, chip emulation,
synth or cartridge export. No public shader/render-target/nine-slice API, arbitrary
polygon renderer, fullscreen mode, touch/IME bridge or controller rumble.

Game UI provides simple retained layout, not keyboard/controller focus, scrolling,
text input or accessibility. Native debug UI is pointer-oriented, not full egui
platform integration. Audio resources lack a public sound-cache-release API.

The editor has actual text/pixel/animation/preset tools, not a full Lua language
server, semantic autocomplete, arbitrary breakpoints, visual scene/ECS editor,
animation-state graph, map painter, expanded asset-import sidecars or dockable panes.
Sprites have no layers/onion skin; clipboard is tab-local. Animation metadata has no
undo. Search is saved-file literal search, not regex or project-wide replacement.
Recent tabs are restored, but unsaved buffers have no crash-recovery journal. Delete
is permanent. Remote source navigation is best-effort.

Export copies the host binary, not every system DLL. No installers, signing,
notarization, consoles, universal or single-file exports. Dependency notice review,
clean-machine acceptance and generated Cargo.lock retention remain release tasks.

## Safety

Project authors and authenticated Link hosts are trusted. Scoped paths/validation
are not an OS sandbox; filesystem races, native decoders and denial of service still
matter. Do not run hostile projects or expose Link as a public production service.
See [security policy](../SECURITY.md) and [Link](kairo-link.md).
