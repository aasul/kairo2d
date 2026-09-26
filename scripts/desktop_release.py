#!/usr/bin/env python3
"""Package already-built native binaries. This script never compiles or renames a foreign binary."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import sys
import tempfile
import zipfile

VERSION = "3.5.0"


def native_executable(path: Path) -> None:
    if not path.is_file() or path.is_symlink():
        raise ValueError(f"Missing regular executable: {path}")
    with path.open("rb") as stream:
        header = stream.read(4)
        if os.name == "nt":
            if not header.startswith(b"MZ"):
                raise ValueError(f"Not a Windows executable: {path.name}")
            stream.seek(0x3C)
            offset = stream.read(4)
            if len(offset) != 4:
                raise ValueError("Truncated PE header")
            stream.seek(int.from_bytes(offset, "little"))
            if stream.read(4) != b"PE\0\0":
                raise ValueError("Missing PE signature")
        elif sys.platform == "darwin":
            if header not in (bytes.fromhex("cffaedfe"), bytes.fromhex("feedfacf"),
                              bytes.fromhex("cafebabe"), bytes.fromhex("cafebabf")):
                raise ValueError(f"Not a native Mach-O executable: {path.name}")
        elif header != b"\x7fELF":
            raise ValueError(f"Not a native ELF executable: {path.name}")
    if os.name != "nt" and not os.access(path, os.X_OK):
        raise ValueError(f"Executable permission is missing: {path.name}")


def package(root: Path, build: Path, output: Path) -> Path:
    suffix = ".exe" if os.name == "nt" else ""
    editor, runtime = build / f"kairo-editor{suffix}", build / f"kairo{suffix}"
    native_executable(editor)
    native_executable(runtime)
    notices = root / "dependency-licenses.json"
    if not notices.is_file():
        raise ValueError("Generate dependency-licenses.json before distributing binaries; see docs/building-games.md")
    data = json.loads(notices.read_text(encoding="utf-8"))
    if not data:
        raise ValueError("Dependency license file is empty")
    system = {"win32": "windows", "darwin": "macos"}.get(sys.platform, sys.platform)
    tag = f"kairo2d-{VERSION}-{system}-{platform.machine().lower()}"
    output.mkdir(parents=True, exist_ok=True)
    archive_path = output / f"{tag}.zip"
    if archive_path.exists():
        raise ValueError(f"Refusing to overwrite {archive_path.name}")
    with tempfile.TemporaryDirectory(prefix="kairo-desktop-") as temporary:
        stage = Path(temporary) / tag
        (stage / "bin").mkdir(parents=True)
        shutil.copy2(editor, stage / f"Kairo{suffix}")
        shutil.copy2(runtime, stage / "bin" / f"kairo{suffix}")
        for name in ("LICENSE", "THIRD_PARTY.md", "README.md", "CHANGELOG.md", "dependency-licenses.json"):
            shutil.copy2(root / name, stage / name)
        if (root / "Cargo.lock").is_file():
            shutil.copy2(root / "Cargo.lock", stage / "Cargo.lock")
        for folder in ("docs", "examples"):
            shutil.copytree(root / folder, stage / folder,
                            ignore=shutil.ignore_patterns(".DS_Store", "Thumbs.db", ".env*", "target", "dist", "__pycache__"))
        (stage / "START-HERE.txt").write_text(
            f"Kairo {VERSION}\n\nLaunch Kairo{suffix} for the editor. Keep bin/kairo{suffix} beside it.\n"
            "The CLI can be run from bin/. Examples are in examples/.\n"
            "This is an unsigned native host build, not an installer or cross-platform package.\n"
            "A build artifact does not establish manual GUI, graphics, or audio acceptance.\n"
            "Source/build commands in README.md refer to the source repository.\n", encoding="utf-8")
        with zipfile.ZipFile(archive_path, "x", zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
            for path in sorted(stage.rglob("*")):
                if path.is_symlink():
                    raise ValueError("Desktop packages cannot contain symlinks")
                if path.is_file():
                    archive.write(path, path.relative_to(stage.parent).as_posix())
    with zipfile.ZipFile(archive_path) as archive:
        if archive.testzip() is not None:
            raise ValueError("Archive integrity check failed")
        expected = {f"{tag}/Kairo{suffix}", f"{tag}/bin/kairo{suffix}", f"{tag}/dependency-licenses.json"}
        if not expected <= set(archive.namelist()):
            raise ValueError("Desktop archive is incomplete")
    digest = hashlib.sha256(archive_path.read_bytes()).hexdigest()
    archive_path.with_suffix(".zip.sha256").write_text(f"{digest}  {archive_path.name}\n", encoding="ascii")
    print(f"Packaged native desktop files: {archive_path}")
    print(f"SHA-256: {digest}")
    return archive_path


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build-dir", type=Path, default=root / "target" / "release")
    parser.add_argument("--output-dir", type=Path, default=root / "dist")
    args = parser.parse_args()
    try:
        package(root, args.build_dir.resolve(), args.output_dir.resolve())
    except (OSError, ValueError, zipfile.BadZipFile) as error:
        print(f"Desktop packaging failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
