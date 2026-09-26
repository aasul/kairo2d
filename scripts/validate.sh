#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
cargo check --workspace --all-targets --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace
python3 scripts/source_audit.py
python3 scripts/smoke.py
