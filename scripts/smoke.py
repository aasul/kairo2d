#!/usr/bin/env python3
"""Exercise the actual executable, never substitute mocked Lua APIs."""
from __future__ import annotations

import argparse
import os
from pathlib import Path
import subprocess
import sys


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--frames", type=int, default=120)
    parser.add_argument("--windowed", action="store_true")
    args = parser.parse_args()
    if args.frames <= 0:
        parser.error("--frames must be positive")
    binary = args.binary or root / "target" / "debug" / ("kairo.exe" if os.name == "nt" else "kairo")
    binary = binary.resolve()
    if not binary.is_file():
        print(f"Executable not found: {binary}. Build with cargo build -p kairo-cli.", file=sys.stderr)
        return 1
    try:
        for project in sorted((root / "examples").iterdir()):
            if not (project / "main.lua").is_file():
                continue
            print(f"Checking {project.name}", flush=True)
            subprocess.run([str(binary), "check", str(project)], check=True, timeout=30)
            command = [str(binary), "run", str(project), "--frames", str(args.frames), "--no-audio", "--no-watch"]
            if not args.windowed:
                command.append("--headless")
            subprocess.run(command, check=True, timeout=120)
    except (OSError, subprocess.SubprocessError) as error:
        print(f"Smoke test failed: {error}", file=sys.stderr)
        return 1
    print("All example smoke tests passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
