# Contributing to Kairo2D

Start by reproducing the current build and reading VALIDATION.md. This repository is
an uncompiled source snapshot; establishing a green native build takes precedence
over adding more surface area. Do not infer test success from test files existing.

Use a focused branch and include a reproducer/test for behavior changes. Pass workspace
formatting, check, tests and strict Clippy with all targets/features. Run real headless
smoke tests after building both executables, and relevant GUI/device/network acceptance
for affected subsystems. Include exact commands and environmental limitations.

Keep Lua APIs small, consistent and backend-independent. Document argument/default,
lifetime, error and reload semantics. Update docs/lua-api.json when a public function
is added, then run `python scripts/generate_api_docs.py`. CI checks the generated
reference with `--check`; the editor reads the same JSON. Examples, lifecycle
semantics and security constraints still need human review. Use opaque handles; never pass raw GPU/physics pointers to Lua. Do not globally
sort translucent draws, bypass scoped paths, block the UI on unbounded work or suppress
lints merely to get green output.

Commit the actual resolved Cargo.lock for a tested release, but never fabricate it.
Do not commit target/dist, local saves, tokens, tool caches or private machine paths.
Review generated dependency notices and asset licenses. Screenshots must come from
the running editor, not a marketing mockup presented as execution evidence.

Pull requests should describe the problem, concrete changes, tests actually run,
known limitations and any compatibility changes. Source contributions and original
assets are MIT-licensed under LICENSE. Follow CODE_OF_CONDUCT.md. Security reports
follow SECURITY.md rather than public exploit disclosures.

For new gameplay systems, prefer bounded device-independent Rust tests and focused
Lua library tests. A Lua test using explicit graphics/physics/audio doubles must be
labeled as such and must not replace native integration coverage. Test inspector
permissions/staleness, profile defaults and protocol rejection before increasing
remote authority. Document an omitted feature rather than shipping an inert control.
