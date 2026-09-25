# Validation - Kairo2D 3.4.0

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
