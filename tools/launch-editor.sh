#!/bin/sh
set -eu
if [ "$#" -gt 1 ]; then
    printf '%s\n' 'Usage: launch-editor.sh [project-directory]' >&2
    exit 2
fi
if [ "$#" -eq 1 ]; then
    game=$(CDPATH= cd -- "$1" && pwd)
    set -- "$game"
fi
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
cargo build -p kairo-cli -p kairo-editor -j 1
exec cargo run -p kairo-editor -- "$@"
