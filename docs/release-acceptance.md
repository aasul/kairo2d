# Native release acceptance

This checklist is a **remaining gate**, not a record of tests already passed. Use
VALIDATION.md for the authoring results. Keep exact OS/toolchain/device details and
all failure output with a release candidate; fix source and rerun affected gates.

## Build gate

```sh
python scripts/source_audit.py
python scripts/generate_api_docs.py --check
cargo fmt --all
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features -j 1
cargo clippy --workspace --all-targets --all-features -j 1 -- -D warnings
cargo test --workspace --all-features -j 1
cargo build --workspace -j 1
python scripts/smoke.py
python scripts/package_smoke.py
```

Do not replace the real runtime smoke tests with Lua test doubles. CI is strict and
does not reformat automatically. Commit a genuinely resolved/tested Cargo.lock before
publishing reproducible native binaries. Review dependency notices for that lockfile.

## Editor and game loop

Create/open each relevant starter, type/edit/save and close/reopen dirty files.
Test dirty tool settings with Save All, external-file conflicts and project-switch/
quit prompts. Verify runtime discovery in a normal/custom target directory and in
`Kairo.exe + bin/kairo.exe` layout. Test Run/Stop/Restart, no orphaned process, syntax
error display, failed candidate retention, successful reload and lost stale session edits.

Exercise all 23 examples graphically; no-audio smoke tests do not prove sound output.
Run Signal Yard: menu, movement/collision, collect eight charges, pause/resume, timeout,
win/restart, high-score persistence, saved schema migration and localization.

## New tooling

Inspector: read-only rejection, numeric range/type checks, current-value conflict,
boolean/vector/color/table/choice edits, reload session rejection, scene controls and
localization selection. Input: held/press/release, brief taps, mouse, controller button/
axis/dead zone, focus loss and reconnect. Particle editor: load/preset conflict/save,
relative texture references, live preview/burst/capacity, native draw with rotation/tint.
Mixer: four bus routing, mute/unmute/target gain, fades, pitch/pan, stop one/all and no-device behavior.

Debug: body shapes/centers/sleep color/velocity and clipped transformed quad bounds.
Maps: animated frames, properties, parallax, visibility/opacity, object collision
rectangles and rejected unsupported formats. Search: ignore rules, bounds, case,
file/line navigation; palette/quick-open/recent tabs/profile toolbar/status telemetry.

## Replay and Link

Register explicit state. Capture/pause/step/restore/resume and verify expected
limitations for particles/audio/unregistered values. Pin a bookmark, let rolling
history evict that time, then restore the pin. Test scene/topology mismatch, eight-pin/
memory eviction, stale IDs, rename/note/delete and metadata-only export.

On localhost then trusted LAN, test two concurrent authenticated testers (up to four
supported), labels, per-client logs/telemetry, auto/manual revision delivery, inspector
edits directed to exactly one peer, disconnect/reconnect, wrong key, old protocol,
malformed/oversized payload and stopped-host cleanup. Verify no token logging. Test
Release profile refusal to join and disabled local tools. No public-internet security
claim follows from a LAN smoke test.

## Distribution

Build actual release binaries, select runtime Release profile, export to a new
output, verify excluded files, relocate and run from an unrelated working directory.
Run `python scripts/package_smoke.py --windowed --binary PATH_TO_RUNTIME` on a
graphical host to test a no-argument packaged launch. Test the packaged editor as well
as Cargo-run development binaries. On a clean Windows machine without Rust installed,
run project creation, Run, save, replay and export. Repeat supported native OS checks;
a Windows result is not evidence of Linux/macOS behavior.
