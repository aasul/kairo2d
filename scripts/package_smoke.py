#!/usr/bin/env python3
"""Test project creation/export using a real built runtime, including relocation."""
from __future__ import annotations

import argparse
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import tomllib


def run(binary: Path, args: list[str], cwd: Path, succeeds: bool = True) -> None:
    completed = subprocess.run([str(binary), *args], cwd=cwd, capture_output=True,
                               text=True, errors="replace", timeout=60)
    if (completed.returncode == 0) != succeeds:
        raise RuntimeError(f"Unexpected exit {completed.returncode} for {' '.join(args)}\n"
                           f"{completed.stdout}\n{completed.stderr}")


def check(binary: Path, windowed: bool) -> None:
    with tempfile.TemporaryDirectory(prefix="kairo-package-test-") as temporary:
        parent = Path(temporary)
        source, output = parent / "Source With Spaces", parent / "Export"
        run(binary, ["new", str(source), "--title", "Package Smoke", "--template", "hello-world"], parent)
        original_config = (source / "kairo.toml").read_bytes()
        run(binary, ["new", str(source)], parent, succeeds=False)
        if (source / "kairo.toml").read_bytes() != original_config:
            raise RuntimeError("new overwrote an existing project")
        (source / "assets" / "probe.txt").write_text("relocation fixture", encoding="ascii")
        (source / "main.lua").write_text('''local frames = 0
function game.load()
    assert(filesystem.read("assets/probe.txt") == "relocation fixture")
end
function game.update(dt)
    frames = frames + 1
    if frames >= 3 then window.close() end
end
function game.draw()
    graphics.clear(0.04, 0.06, 0.09, 1)
    graphics.rectangle("fill", 20, 20, 32, 32)
end
''', encoding="utf-8")
        (source / "target").mkdir()
        (source / "target" / "not-an-asset").write_text("excluded", encoding="ascii")
        (source / ".env").write_text("fixture-not-a-secret", encoding="ascii")
        (source / "session.token").write_text("fixture-not-a-secret", encoding="ascii")
        run(binary, ["check", str(source)], parent)
        run(binary, ["build", str(source), "--output", str(output)], parent)
        run(binary, ["build", str(source), "--output", str(output)], parent, succeeds=False)
        executable = output / ("PackageSmoke.exe" if os.name == "nt" else "PackageSmoke")
        if hashlib.sha256(executable.read_bytes()).digest() != hashlib.sha256(binary.read_bytes()).digest():
            raise RuntimeError("exported runtime differs from input executable")
        marker = tomllib.loads((output / "kairo-package.toml").read_text(encoding="utf-8"))
        if marker != {"format": 1, "project": "game", "profile": "release"}:
            raise RuntimeError("unexpected package marker")
        for name in ("target", ".env", "session.token"):
            if (output / "game" / name).exists():
                raise RuntimeError(f"excluded project file was packaged: {name}")
        for name in ("KAIRO-LICENSE.txt", "THIRD_PARTY.md", "game/main.lua", "game/assets/probe.txt"):
            if not (output / name).is_file():
                raise RuntimeError(f"missing package entry: {name}")
        relocated = parent / "Relocated Package"
        shutil.move(output, relocated)
        shutil.rmtree(source)
        executable = relocated / executable.name
        run(executable, ["run", str(relocated / "game"), "--headless", "--frames", "10", "--no-audio", "--no-watch"], parent)
        if windowed:
            # No arguments exercises automatic marker discovery, not explicit project selection.
            run(executable, [], parent)
    print("Real CLI creation/check/export/relocation tests passed.")
    if not windowed:
        print("No-argument graphical package launch was not tested; use --windowed on a graphical host.")


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=root / "target" / "debug" / ("kairo.exe" if os.name == "nt" else "kairo"))
    parser.add_argument("--windowed", action="store_true")
    args = parser.parse_args()
    binary = args.binary.resolve()
    if not binary.is_file():
        print(f"Build the actual Kairo runtime first; missing {binary}", file=sys.stderr)
        return 1
    try:
        check(binary, args.windowed)
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"Package smoke test failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
