# Validation - Kairo2D 3.4.0

## Build status

This is a source snapshot of Kairo2D 3.4.0. It has not been compiled or verified as a finished release.

Version 3.4.0 is based on the 3.3.0 Feature UI codebase. The existing UI work, boxed enum fixes, explicit float types, runtime diagnostics, and packaging support are still included.

The Link protocol added in 3.4.0 is not compatible with 3.3 clients.

The repository currently contains 348 files across ten Rust crates. Compared with the 238-file Feature UI baseline, 110 files were added, 73 were changed, and none were removed.

The project includes:

- 23 examples
- 16 embedded templates
- 98 Rust source files
- 146 Rust test functions

No Rust tests were run in this environment.

Validation was done on 2026-09-18 using a Linux environment that did not have Cargo, rustc, rustfmt, or PowerShell installed.

A Rust toolchain was not found locally. An attempt to download one also failed because DNS resolution was unavailable.

Because of that, the project was not compiled and Cargo could not resolve dependencies. Clippy, rustfmt, Cargo tests, and Cargo build checks could not be run either.

I also tried to install a Rust parser for an extra syntax check, but that failed for the same network reason.

The archive does not contain a generated `Cargo.lock`, compiled editor or runtime binaries, successful CI output, screenshots, font files, or machine-specific build artifacts.

Until the project is built on a machine with a working Rust toolchain, compile errors, dependency mismatches, formatting issues, Clippy warnings, and runtime bugs are still possible.

## Commands run

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Could not run. `cargo: not found` (exit 127) |
| `cargo check --workspace` | Could not run. `cargo: not found` (exit 127) |
| `cargo test --workspace --all-features -j 1` | Could not run. `cargo: not found` (exit 127) |
| `cargo clippy --workspace --all-targets --all-features -j 1 -- -D warnings` | Could not run. `cargo: not found` (exit 127) |
| `cargo build --workspace -j 1` | Could not run. `cargo: not found` (exit 127) |
| `python scripts/source_audit.py` | Passed |
| `python scripts/generate_api_docs.py --check` | Passed. 157 API entries matched |
| `python scripts/lua_sanity.py` | Passed using Lua 5.4 |
| `python scripts/smoke.py` | Failed because the runtime executable was not available |
| `python scripts/package_smoke.py` | Failed because the runtime executable was not available |
| Python AST parsing of seven Python scripts | Passed syntax checks |
| YAML parsing of seven workflow/template files | Passed syntax checks |
| `sh -n` on three shell scripts | Passed syntax checks |
| Windows build and launch scripts | Reviewed only. Windows and PowerShell were not available |

The Cargo checks failed at startup because Cargo was missing. These failures were not treated as successful validation.

The Windows launcher still has its `-Validate` checks, and the CI configuration still contains the expected native build checks.

## Source checks

The source audit completed successfully.

It parsed 63 TOML files and 11 JSON files.

It also checked:

- all ten workspace members
- shared and path dependencies
- 80 embedded Rust file paths
- consistency between examples and the 16 embedded templates
- 157 API metadata entries
- local Markdown links
- 13 PNG files
- 7 WAV files

These checks are useful for catching broken paths, invalid metadata, and obvious repository issues, but they do not replace Cargo dependency resolution or an actual build.

The main new and changed systems were also reviewed with the code that uses them. This included:

- scene callbacks and runtime lifecycle
- action snapshots and input edges
- inspector updates and session handling
- particle runtime and editor preset sharing
- audio routing
- Link protocol handling
- bookmark timeline replacement
- runtime profiles
- package markers
- generated API documentation

This was source review only. It does not prove that the Rust code type-checks or that the implementation is secure.

The packaged source was also checked for common unwanted files and paths.

No symlinks, `target` directories, `.git` directories, cache directories, external font binaries, or obvious local machine paths were found.

Known generated files and common credential patterns were also checked.

This is a basic repository hygiene check, not a full secret scan or supply-chain audit.

## Lua checks

Lua 5.4 was available, so the Lua side of the project could be tested more thoroughly.

A total of 55 Lua files and 157 API snippets compiled successfully.

That only confirms the Lua syntax and behavior tested by the scripts. It does not confirm that the Rust functions exposed to Lua compile or behave correctly.

The Lua tests covered:

- scene lifecycle
- queued scene changes
- push and pop pause overlays
- update and draw boundaries
- transition recovery
- animator conditions and completion transitions
- animator pause and speed handling
- localization fallback and interpolation
- localization path validation
- prefab deep overrides
- prefab instance isolation
- ordered save migrations
- incompatible save-version rejection
- inspector registration
- in-place inspector table updates
- retained UI click and layout behavior
- retained UI removal
- Replay table serialization
- Replay in-place restoration
- rejection of unsupported Replay aliases, cycles, types, and excessive depth
- collision helper math
- movement normalization and bounds
- Link state transfer hooks

Some of these tests use small test doubles in place of native systems such as graphics, windowing, audio, physics, input, assets, and particles.

With those substitutes in place, the following game logic checks passed:

- Breakout ran for 600 update and draw iterations
- Micro ran for 1800 ticks
- all eight new 3.4 starter examples ran through their scripted checks
- Signal Yard ran for 600 ticks and covered menu flow, pause and resume, collecting all eight charges, winning, saving, and restarting

The tests used the JSON and TOML files included with the examples.

Font fallback behavior and gamepad fallback behavior were also checked without a real font renderer or physical controller.

These tests do not cover native rendering, real physics collisions, audio output, controller hardware, inspector networking, or editor GUI behavior.

## Rust tests

There are 146 Rust test functions in the source tree, but none of them were executed.

They cover areas including:

- stale handles
- configuration and path validation
- scene and action integration
- inspector validation
- inspector write permissions
- RNG and replay behavior
- kinematics
- pinned snapshot limits
- snapshot restoration
- particle capacity and lifetime handling
- mixer state without audio devices
- Tiled data
- runtime profiles
- search
- packaging
- authenticated Link tester channels

The presence of these tests only means the test cases exist. It does not mean they pass.

## Before release

The project still needs a full native validation pass on a machine with a working Rust toolchain.

At minimum, run:

- `cargo fmt --all -- --check`
- `cargo check --workspace`
- `cargo test --workspace --all-features -j 1`
- `cargo clippy --workspace --all-targets --all-features -j 1 -- -D warnings`
- `cargo build --workspace -j 1`

Any issues found there should be fixed before treating this as a release build.

The editor should also be tested manually for:

- startup
- pixel editing
- text editing
- Run, Stop, and Restart
- the new editor tools
- local inspector writes
- remote inspector writes
- controller input
- physics
- audio
- Link authentication
- multiple Link clients
- Replay restore
- Replay pins
- runtime profiles
- standalone exports
- relocated standalone exports

The Windows package layout should also be tested on a clean Windows machine without Rust installed.

Release checks are documented in `docs/release-acceptance.md`.

Known limitations are listed in `docs/known-limitations.md`.

Current omissions include:

- scene fades
- animator graph UI
- arbitrary debugger or eval support
- LAN discovery
- screenshot transfer
- portable bookmark import
- fully deterministic rewind
- expanded import sidecars
- custom audio buses
- streaming audio
- plugin loading
- cross-compilation service

Link and Replay are still experimental and have not been security audited.
