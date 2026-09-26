# Validation - Kairo2D 3.4.0

## Relay Dusk game follow-up — 2026-09-26

`examples/relay-dusk` is a new playable project using the current public Lua
runtime. It includes original generated sprites and sounds, a three-relay
survival mission, upgrades, enemy waves, a boss, extraction, scoring and saves.
The engine APIs are unchanged.

| Command or check | Result on this Windows host |
| --- | --- |
| `cargo fmt --all -- --check` | Passed. |
| `cargo check --workspace --all-targets --all-features` | Passed. |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Passed. |
| `cargo test --workspace --all-features` | Passed, including `relay_dusk_starts_fights_and_pauses_headlessly`. |
| `cargo build --workspace` | Passed. |
| `target/debug/kairo.exe check examples/relay-dusk` | Passed after the final Lua change. |
| `scripts/source_audit.py` | Passed with bundled Python: 176 API entries, 22 PNGs and 15 WAVs. |
| `scripts/generate_api_docs.py --check` | Passed. |
| `scripts/smoke.py` | Passed for all 24 examples after the final Lua change. |
| `scripts/package_smoke.py` | Passed for CLI creation, export and relocated headless execution. |
| `target/debug/kairo.exe build examples/relay-dusk --output dist/RelayDusk` | Passed; created ignored `dist/RelayDusk/RelayDusk.exe` with its project data and release package marker. A no-argument graphical launch opened a responding `Relay Dusk` window. |
| `git diff --check` | Passed. |
| Visible runtime and editor launch | `kairo.exe run examples/relay-dusk --no-watch` and `kairo-editor.exe examples/relay-dusk` opened responding windows. The initial runtime launch exposed an audio playback restriction in `game.load`; music was moved into the first menu update. The development game window was then replaced with the responding standalone package window. |

The integration test starts the mission, moves and fires for 180 frames, checks
that drawing commands were produced, and pauses. It does not complete all three
relays or the boss fight. Visible window startup does not establish visual quality
or physical controller and audio-output behavior; those need hands-on acceptance.
The host emitted non-fatal Vulkan validation-layer and D3D12 debug-interface
warnings during the visible launch.

## Local Windows follow-up — 2026-09-26

This section describes the current checkout after adding the validated node hierarchy,
JSON scene loading, attached node scripts, hierarchical JSON prefabs,
node/global signals, Lua bindings, visual `.scene` resource editing, `.prefab`
text editing, an updated Scene Workshop starter, and a Windows Link socket fix.
The follow-up also adds native scene drawing, Rapier bodies and contact dispatch,
AudioSource playback management, Control pointer interaction, typed editor
properties and marker dragging. The contact regression test covers both the
`collision` signal and attached `onCollision(self, other)` callback.
The 2026-09-18 source-only report below is historical; its statements that Cargo and
native tests were unavailable do not describe this follow-up. No release is claimed.

| Command | Result on this Windows host |
| --- | --- |
| `cargo fmt --all -- --check` | Passed. |
| `cargo check --workspace --all-targets --all-features` | Passed. |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Passed. |
| `cargo test -p kairo-lua native_body_contacts_emit_validated_scene_node_signals --all-features` | Passed after strengthening assertions for the signal and attached script callback. |
| `cargo test --workspace --all-features` | Passed after the latest contact, Control and test changes, including graph, Lua scene, native drawing/physics/audio, editor, Scene Workshop and Link tests. |
| `cargo build --workspace` | Passed; editor and runtime binaries built. |
| `scripts/source_audit.py` | Passed with bundled Python; 176 API entries synchronized. |
| `scripts/generate_api_docs.py --check` | Passed with bundled Python. |
| `scripts/package_smoke.py` | Passed; real CLI project creation, export and relocated headless game execution. |
| `scripts/smoke.py` | Passed for all 23 examples using the built runtime with headless, no-audio execution. |
| `scripts/lua_sanity.py` | Could not run the optional Lua-only check: this host lacks a native Lua 5.4 shared library. The Rust Lua runtime tests and example smoke tests did run. |
| `git diff --check` | Passed. |

`target/debug/kairo.exe check examples/scenes` and `target/debug/kairo.exe run
examples/scenes --frames 120 --no-audio --no-watch --headless` passed. A focused
Rust test entered the workshop's gameplay scene and confirmed its player script and
two prefab marker scripts produced three quads. A local debug-mode 5,000-node
graph creation and transform query completed in 42.2003 ms; this is one host
measurement, not a frame-rate or release-build claim.

The pre-change format check found only existing `kairo-editor/src/hub.rs` formatting,
which was corrected. The pre-change workspace check and strict Clippy passed. The
first full test run reached an existing `kairo-link` Windows TCP test failure
(connection reset/abort, OS errors 10054/10053). Accepted sockets now explicitly use
blocking mode; the isolated test and final full workspace run pass.
During the editor hierarchy addition, one new test initially kept a node handle
across Undo, which reconstructs the graph and invalidates old handles. The test
was corrected to reacquire nodes by path; its focused rerun and the final full
workspace run pass.

The editor GUI was not opened for visual acceptance during this continuation. A
previous bounded editor startup remained alive for five seconds, which is only a
startup smoke check. GPU drawing, actual audio-device output, physical controllers,
graphical export launch and cross-platform CI were not verified on this host.
Native camera, sprite, Control, physics and AudioSource integration is exercised by
headless tests, not by visual or device acceptance. The scene resource tab has a
tree, typed inspector, node-marker dragging, bounds and camera frame preview; its
Undo/Redo history is local to the scene tab. Area2D sensors and trigger enter/exit,
fixedUpdate scheduling, contact normals, contacts with manually created physics
bodies, authored scene animation/particles/tilemaps, runtime UI focus/layout/text
input, render targets/shaders, and the complete public-API demo remain incomplete.

## Result and provenance

This is an **uncompiled development source snapshot**, not a verified executable
release. It extends the latest `kairo2d-3.3.0-feature-ui.zip` supplied in this
conversation, not a clean rewrite or the earlier 0.2 baseline. Existing feature UI,
boxed enum fixes, explicit float types, runtime diagnostics and packaging services
are retained. The new Link protocol is intentionally incompatible with 3.3 clients.

The repository contains 348 files across ten Rust crates: 110 added, 73 changed and
none removed relative to the 238-file Feature UI baseline. There are 23 examples,
16 embedded templates, 98 Rust source files and 146 Rust test functions written.
**Zero Rust tests executed in this environment.**

Validation was performed on 2026-09-18 in the Linux authoring environment. Cargo,
rustc, rustfmt and PowerShell were unavailable; bounded toolchain discovery found
no installed toolchain, and a toolchain download attempt failed DNS resolution.
No dependency resolution or Rust compilation occurred. A separate attempt to obtain
a Rust syntax-parser dependency also failed; no substitute parser success is claimed.

There is no native editor/runtime, fabricated Cargo.lock, successful CI run, GUI
screenshot, font file or machine-specific build output in the archive. Compilation,
Clippy, formatting, dependency API and runtime defects may remain.

## Commands actually attempted

| Command | Observed result |
| --- | --- |
| `cargo fmt --all -- --check` | Could not start: `cargo: not found`, exit 127. |
| `cargo check --workspace` | Could not start: `cargo: not found`, exit 127. |
| `cargo test --workspace --all-features -j 1` | Could not start: `cargo: not found`, exit 127. |
| `cargo clippy --workspace --all-targets --all-features -j 1 -- -D warnings` | Could not start: `cargo: not found`, exit 127. |
| `cargo build --workspace -j 1` | Could not start: `cargo: not found`, exit 127. |
| `python scripts/source_audit.py` | Passed. |
| `python scripts/generate_api_docs.py --check` | Passed: 157 API entries synchronized. |
| `python scripts/lua_sanity.py` | Passed using native Lua 5.4; distinctions below apply. |
| `python scripts/smoke.py` | Exit 1: actual runtime executable absent; no examples executed through Rust. |
| `python scripts/package_smoke.py` | Exit 1: actual runtime executable absent; no native export/relocation run. |
| Python AST parsing of seven Python scripts | Passed, syntax only. |
| YAML parsing of seven workflow/template files | Passed, syntax only; no GitHub Actions execution. |
| `sh -n` for three shell scripts | Passed, syntax only. |
| Windows build/launch scripts | Inspected, not executed; PowerShell/Windows unavailable. |

The initial Cargo baseline gates and final native gates both encountered the missing
toolchain. The source was not declared clean on the basis of these failures. The
Windows launcher with `-Validate` and configured CI retain strict native gates.

## Static/source checks that passed

The source audit parsed 63 TOML documents and 11 JSON documents; verified ten workspace
members, shared/path dependency references, Rust embedded-file paths (80), example/
template consistency (16 templates), 157 API metadata entries and local Markdown
links. It independently checked 13 PNG and seven WAV files. Cargo manifest syntax
and internal path checks are not Cargo dependency resolution or `cargo metadata`.

New systems and their consumers were reviewed together: scene callbacks/runtime
lifecycle, action snapshots/input edges, inspector typed updates/session checks,
particle runtime/editor preset sharing, audio routes, Link protocol exchange,
bookmark timeline replacement, runtime profiles/package markers and generated docs.
This is source review, not a proof of Rust type correctness or security.

No symlinks, target/.git/cache directories, external font binaries or common local
machine-path strings were found in the packaged source. Known credential/generated
artifact patterns are excluded. This is a targeted hygiene check, not an exhaustive
secret scanner or supply-chain audit.

## Executed Lua checks

Native Lua 5.4 compiled 55 Lua files and 157 API snippets. Syntax success does not
establish that the Rust-side API implementations compile or behave correctly.

The actual bundled Lua implementations passed checks for:

- Scene lifecycle, queued changes, push/pop pause overlays, update/draw boundaries
  and bounded transition recovery.
- Animator predicate/completion transitions, pause and speed behavior.
- Localization fallback/interpolation/path validation; prefab deep overrides and
  instance isolation; ordered save migrations and incompatible-version rejection.
- Explicit inspector registration and in-place table updates. **Host-side Rust
  validation was doubled here**, not executed.
- Retained UI click/layout/removal behavior and Replay plain-table serialization,
  in-place restoration and unsupported alias/cycle/type/depth rejection.
- Collision helper math, movement normalization/bounds and Link state-transfer hooks.

Explicit native-boundary test doubles were used for graphics/window/audio/physics/
input/assets/particles as necessary. With those doubles, game-logic checks passed for
Breakout (600 update/draw iterations), Micro (1800 ticks), the eight new 3.4 starters,
and Signal Yard (600 ticks plus menu, pause/resume, collection of all eight charges,
win/save and restart). The fixtures read the actual bundled JSON/TOML example data.
Font and gamepad fallbacks were checked without a real font renderer/controller.

These results **do not validate native physics collisions, GPU rendering, audio
routing/output, controller devices, inspector networking or GUI behavior**. Tests
are explicitly labeled in their output and do not replace real runtime smoke scripts.

## Native tests authored but not run

The 146 Rust test functions cover existing and new areas including stale handles,
configuration/path guards, scene/action integration, inspector validation and write
permissions, RNG/replay/kinematics, pinned snapshot limits and restoration, particle
capacity/lifetimes, device-free mixer state, Tiled data, profiles, search, package
logic and two authenticated Link tester channels. Their presence is not test success.

## Archive checks

The delivered ZIP is built with required-entry checks, deterministic source paths,
CRC verification and a SHA-256 sidecar. It is independently extracted; all 348 file
bytes are compared with the repository; source/API and Lua checks are rerun from
that extracted copy. No source build/cache output is distributed.

The archive's checksum is in `kairo2d-3.4.0.zip.sha256`, alongside the ZIP. It is not
embedded into this report inside the same ZIP (which would create a self-referential
checksum). No host-native release archive was produced.

## Explicit remaining release gates

Run the full build/test/Clippy/format workflow on a supported native machine; fix
all reported source issues. Test editor startup, actual pixel/text editing, Run/
Stop/Restart, all new tools, local/remote inspector writes, controller/physics/audio,
Link authentication and multiple clients, Replay restore/pins, profiles and relocated
standalone exports. Repeat the intended Windows layout on a clean machine without
Rust. Follow `docs/release-acceptance.md` and record exact results.

Scope omissions are recorded in `docs/known-limitations.md`: no scene fades, animator
graph UI, arbitrary debugger/eval, LAN discovery, screenshot transfer, portable
bookmark import, universal deterministic rewind, expanded import sidecars, custom
buses, streaming audio, plugin loader or cross-compilation service. Link and Replay
remain experimental and unaudited. None of these is represented by a fake API/button.
