#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features -j 1
cargo clippy --workspace --all-targets --all-features -j 1 -- -D warnings
cargo test --workspace --all-features -j 1
cargo build --workspace -j 1
python3 scripts/source_audit.py
python3 scripts/smoke.py
python3 scripts/package_smoke.py
