# Development

Use the root README and release acceptance checklist for prerequisites and native
validation. The existing Rust tests cover independent services, Lua integration,
editor text/pixel operations and Link protocol cases; they must actually be run.
No test count alone establishes reliability.

```sh
cargo fmt --all
cargo check --workspace --all-targets --all-features -j 1
cargo clippy --workspace --all-targets --all-features -j 1 -- -D warnings
cargo test --workspace --all-features -j 1
cargo build --workspace -j 1
python scripts/smoke.py
```

Source-only checks: `python scripts/source_audit.py` parses manifests/data, verifies
references, template consistency and assets. Optional `python scripts/lua_sanity.py`
uses a native Lua 5.4 shared library to compile scripts and run labeled Lua-only tests.
It does not invoke Rust and uses explicit doubles for engine boundaries.

When adding a Lua API, implement validation at the Rust boundary, add tests, document
its exact semantics and update `docs/lua-api.json` snippets and `.luarc.json` globals.
When adding format/backend support, reject unsupported inputs explicitly. Never
create a disabled UI button or success-returning placeholder to imply support.

Avoid broad lint suppression. Keep painter-order batching, scoped filesystem writes,
atomic saves, bounded queues and candidate-swap boundaries intact. Do not export
backend handles to scripts. Prefer a concrete service over a trait without a user.

## Shared API metadata

Edit docs/lua-api.json for function signatures/descriptions/examples, then run
`python scripts/generate_api_docs.py`. The generator also writes the LuaLS
declarations for profiler and debug functions from this metadata. The source
audit checks bindings against metadata; the generated reference and declarations
must pass `--check`. Both checks are source
consistency checks, not a proof of native call correctness.
