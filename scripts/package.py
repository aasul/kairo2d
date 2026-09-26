#!/usr/bin/env python3
"""Package repository sources. Compilation must be validated separately."""
from __future__ import annotations

import argparse
import hashlib
from pathlib import Path
import sys
import zipfile

VERSION = "3.5.0"
SKIP_PARTS = {".git", "target", "dist", "__pycache__", ".idea", ".vscode", ".venv"}
SKIP_NAMES = {".DS_Store", "Thumbs.db", ".env", "session-token.txt", "dependency-licenses.json"}
REQUIRED = {
    "crates/kairo-core/src/inspector.rs", "crates/kairo-core/src/actions.rs",
    "crates/kairo-core/src/particles.rs", "crates/kairo-core/src/bookmarks.rs",
    "crates/kairo-core/src/profile.rs", "crates/kairo-lua/src/builtin/scene.lua",
    "crates/kairo-lua/src/builtin/animator.lua", "crates/kairo-lua/tests/workflows.rs",
    "crates/kairo-editor/src/develop.rs", "crates/kairo-editor/src/particle_tool.rs",
    "crates/kairo-editor/src/commands.rs", "crates/kairo-editor/src/inspector.rs",
    "examples/top-down/main.lua", "examples/platformer/main.lua",
    "docs/scenes.md", "docs/input-actions.md", "docs/live-inspector.md",
    "docs/api-reference.md", "scripts/generate_api_docs.py",

    "Cargo.toml", "README.md", "LICENSE", "VALIDATION.md", "CONTRIBUTING.md",
    "CODE_OF_CONDUCT.md", "SECURITY.md", "CHANGELOG.md", ".github/workflows/ci.yml",
    "docs/getting-started.md", "docs/lua-api.md", "docs/architecture.md",
    "crates/kairo-cli/src/main.rs", "crates/kairo-lua/src/lib.rs",
    "crates/kairo-render/src/sprite.wgsl", "examples/hello-world/main.lua",
    "examples/sprites/assets/characters.png", "examples/movement/main.lua",
    "examples/audio/assets/chime.wav", "examples/physics/main.lua",
    "crates/kairo-editor/src/main.rs", "crates/kairo-editor/src/code.rs",
    "crates/kairo-editor/src/pixels.rs", "crates/kairo-project/src/package.rs",
    "crates/kairo-project/src/files.rs", "docs/editor.md", "docs/cli.md",
    "docs/configuration.md", "docs/hot-reload.md", "docs/building-games.md",
    "docs/lua-api.json", "examples/breakout/main.lua", "examples/breakout/collision.lua",
    "examples/breakout/assets/ball.png", "examples/breakout/assets/bounce.wav",
    ".github/workflows/desktop-release.yml", "scripts/desktop_release.py",
    "scripts/source_audit.py", "scripts/lua_sanity.py",
    "crates/kairo-link/src/wire.rs", "crates/kairo-replay/src/lib.rs",
    "crates/kairo-render/src/micro.wgsl", "crates/kairo-assets/src/fonts.rs",
    "crates/kairo-lua/tests/v3.rs", "tools/build-windows.ps1",
    "docs/kairo-link.md", "docs/kairo-replay.md", "docs/kairo-micro.md",
    "examples/animation/main.lua", "examples/tilemap/main.lua", "examples/link/main.lua",
    "examples/replay/main.lua", "examples/ui/main.lua", "examples/micro/main.lua",
}


def source_files(root: Path):
    for path in sorted(root.rglob("*")):
        relative = path.relative_to(root)
        if any(part in SKIP_PARTS for part in relative.parts):
            continue
        if path.name in SKIP_NAMES or path.name.startswith(".env."):
            continue
        if path.name.endswith((".zip", ".zip.sha256", ".pyc", ".swp", ".swo", ".kairo-token", ".link-token", ".token", "~")):
            continue
        if path.is_symlink():
            raise ValueError(f"Refusing to package symlink: {relative}")
        if path.is_file():
            yield path, relative.as_posix()


def package(root: Path, output: Path) -> int:
    files = list(source_files(root))
    names = {name for _, name in files}
    missing = REQUIRED - names
    if missing:
        raise ValueError(f"Missing required repository files: {', '.join(sorted(missing))}")
    output.parent.mkdir(parents=True, exist_ok=True)
    prefix = f"kairo2d-{VERSION}/"
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for path, relative in files:
            info = zipfile.ZipInfo(prefix + relative, date_time=(2026, 9, 18, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 3
            executable = relative.endswith(".sh") or (relative.startswith("scripts/") and relative.endswith(".py"))
            info.external_attr = (0o100755 if executable else 0o100644) << 16
            archive.writestr(info, path.read_bytes())
    with zipfile.ZipFile(output) as archive:
        damaged = archive.testzip()
        if damaged:
            raise ValueError(f"Archive CRC failure: {damaged}")
        if not {prefix + name for name in REQUIRED}.issubset(archive.namelist()):
            raise ValueError("Archive required-entry check failed")
    digest = hashlib.sha256(output.read_bytes()).hexdigest()
    output.with_suffix(output.suffix + ".sha256").write_text(f"{digest}  {output.name}\n", encoding="ascii")
    print(f"Packaged {len(files)} files: {output.name} ({output.stat().st_size:,} bytes)")
    print(f"SHA-256: {digest}")
    return len(files)


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=root.parent / f"kairo2d-{VERSION}.zip")
    args = parser.parse_args()
    try:
        package(root, args.output.resolve())
    except (OSError, ValueError, zipfile.BadZipFile) as error:
        print(f"Packaging failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
