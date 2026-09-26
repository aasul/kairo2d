#!/usr/bin/env python3
"""Check repository consistency without claiming Rust compilation or execution."""
from __future__ import annotations

import json
from pathlib import Path
import re
import sys
import tomllib
import wave
import struct
import zlib


def png_check(path: Path) -> None:
    raw = path.read_bytes()
    if not raw.startswith(b"\x89PNG\r\n\x1a\n"):
        raise ValueError(f"Invalid PNG signature: {path}")
    offset, seen_end, image_data = 8, False, bytearray()
    while offset + 12 <= len(raw):
        length = struct.unpack(">I", raw[offset:offset + 4])[0]
        kind = raw[offset + 4:offset + 8]
        end = offset + length + 8
        if end + 4 > len(raw):
            raise ValueError(f"Truncated PNG: {path}")
        payload = raw[offset + 8:end]
        if zlib.crc32(kind + payload) != struct.unpack(">I", raw[end:end + 4])[0]:
            raise ValueError(f"PNG CRC mismatch: {path}")
        if kind == b"IHDR":
            width, height = struct.unpack(">II", payload[:8])
            if not (1 <= width <= 8192 and 1 <= height <= 8192):
                raise ValueError(f"Invalid texture dimensions: {path}")
        elif kind == b"IDAT":
            image_data.extend(payload)
        elif kind == b"IEND":
            seen_end = True
            break
        offset = end + 4
    if not seen_end or not image_data or not zlib.decompress(image_data):
        raise ValueError(f"Incomplete PNG: {path}")


def audit(root: Path) -> dict[str, int]:
    paths = [p for p in root.rglob("*") if p.is_file() and not any(part in {"target", "dist", ".git", "__pycache__"} for part in p.relative_to(root).parts)]
    counters = {"TOML": 0, "JSON": 0, "Rust include paths": 0, "Markdown local links": 0,
                "PNG": 0, "WAV": 0, "templates": 0, "API palette entries": 0}
    manifest = tomllib.loads((root / "Cargo.toml").read_text())
    version = manifest["workspace"]["package"]["version"]
    dependencies = manifest["workspace"]["dependencies"]
    names = set()
    for member in manifest["workspace"]["members"]:
        member_path = root / member
        package = tomllib.loads((member_path / "Cargo.toml").read_text())
        name = package["package"]["name"]
        if name in names:
            raise ValueError(f"Duplicate workspace crate {name}")
        names.add(name)
        if not package["package"].get("version", {}).get("workspace"):
            raise ValueError(f"{name} must inherit the workspace version")
        if not (member_path / "src/lib.rs").is_file() and not (member_path / "src/main.rs").is_file():
            raise ValueError(f"Missing crate entrypoint: {name}")
        for section in ("dependencies", "dev-dependencies", "build-dependencies"):
            for key, value in package.get(section, {}).items():
                if isinstance(value, dict) and value.get("workspace") and key not in dependencies:
                    raise ValueError(f"Unresolved workspace dependency: {name} -> {key}")
    for name, dependency in dependencies.items():
        if isinstance(dependency, dict) and "path" in dependency:
            if not (root / dependency["path"] / "Cargo.toml").is_file():
                raise ValueError(f"Missing local dependency {name}")
            if dependency.get("version") != version:
                raise ValueError(f"Stale dependency version for {name}")
    for path in paths:
        if path.suffix == ".toml":
            tomllib.loads(path.read_text(encoding="utf-8"))
            counters["TOML"] += 1
        elif path.suffix in {".json", ".tmj", ".tsj", ".scene", ".prefab"}:
            json.loads(path.read_text(encoding="utf-8"))
            counters["JSON"] += 1
        elif path.suffix == ".rs":
            source = path.read_text(encoding="utf-8")
            for included in re.findall(r'include_(?:str|bytes)!\(\s*"([^"]+)"\s*\)', source):
                if not (path.parent / included).is_file():
                    raise ValueError(f"Missing include: {path.relative_to(root)} -> {included}")
                counters["Rust include paths"] += 1
        elif path.suffix == ".md":
            source = path.read_text(encoding="utf-8")
            for target in re.findall(r'\[[^\]\n]*\]\(([^\s)]+)(?:\s+"[^"]*")?\)', source):
                if target.startswith(("https:", "http:", "mailto:", "#")):
                    continue
                local = target.split("#", 1)[0]
                if local and not (path.parent / local).exists():
                    raise ValueError(f"Broken local link: {path.relative_to(root)} -> {target}")
                counters["Markdown local links"] += 1
        elif path.suffix == ".png":
            png_check(path)
            counters["PNG"] += 1
        elif path.suffix == ".wav":
            with wave.open(str(path), "rb") as sound:
                frames = sound.getnframes()
                if frames <= 0 or sound.getframerate() <= 0:
                    raise ValueError(f"Empty WAV: {path}")
                if len(sound.readframes(frames)) != frames * sound.getnchannels() * sound.getsampwidth():
                    raise ValueError(f"Truncated WAV: {path}")
            counters["WAV"] += 1
    for template in (root / "crates/kairo-project/templates").iterdir():
        if not (template / "main.lua").is_file() or not (template / "kairo.toml").is_file():
            raise ValueError(f"Missing template entrypoint/config: {template.name}")
        if template.name != "empty":
            for path in template.rglob("*"):
                if path.is_file():
                    example = root / "examples" / template.name / path.relative_to(template)
                    if not example.is_file() or path.read_bytes() != example.read_bytes():
                        raise ValueError(f"Template differs from example: {path.relative_to(root)}")
        counters["templates"] += 1
    api = json.loads((root / "docs/lua-api.json").read_text())
    signatures = set()
    for entry in api:
        if set(entry) != {"signature", "snippet", "description"} or not all(entry.values()):
            raise ValueError("Malformed API palette entry")
        if entry["signature"] in signatures:
            raise ValueError(f"Duplicate API signature: {entry['signature']}")
        signatures.add(entry["signature"])
        signature = entry["signature"].split("(", 1)[0]
        if " / " in signature:
            source = (root / "crates/kairo-lua/src/scene_graph.rs").read_text()
            for field_signature in signature.split(" / "):
                module, field = field_signature.split(".", 1)
                if module != "node" or not re.search(r'\.add_field_function_(?:get|set)\(\s*"' + re.escape(field) + r'"', source):
                    raise ValueError(f"Palette field is not registered: {field_signature}")
            counters["API palette entries"] += 1
            continue
        method = ":" in signature
        module, function = signature.split(":" if method else ".")
        sources = {
            "graphics": ["graphics.rs", "fonts.rs"], "audio": ["audio.rs"], "physics": ["physics.rs"],
            "animation": ["animation.rs"], "tilemap": ["tilemap.rs"], "gamepad": ["gamepad.rs"],
            "save": ["save.rs", "builtin/save_migrations.lua"], "profiler": ["profiler.rs"], "random": ["replay.rs"],
            "replay": ["builtin/replay.lua"], "ui": ["builtin/ui.lua"], "debugui": ["debugui.rs"],
            "scene": ["builtin/scene.lua"], "input": ["actions.rs"], "inspector": ["builtin/inspector.lua"],
            "animator": ["builtin/animator.lua"], "particles": ["particles.rs"], "debug": ["profiler.rs"],
            "prefab": ["builtin/prefab.lua"], "localization": ["builtin/localization.lua"],
            "node": ["scene_graph.rs"], "Events": ["builtin/events.lua"],
            "Animator": ["builtin/animator.lua"], "Emitter": ["particles.rs"], "TileMap": ["tilemap.rs"],
        }.get(module, ["services.rs"])
        source = "\n".join((root / "crates/kairo-lua/src" / file).read_text() for file in sources)
        if method:
            rust = re.search(r'\.add_method(?:_mut)?\(\s*"' + re.escape(function) + r'"', source)
            lua = re.search(r'function\s+Machine:' + re.escape(function) + r'\(', source) if module == "Animator" else None
        else:
            rust = re.search(r'\.set\(\s*"' + re.escape(function) + r'"', source)
            lua = re.search(r'function\s+' + re.escape(module + "." + function) + r'\(', source)
            if module in {"input", "debug"} and not rust:
                rust = re.search(r'\("' + re.escape(function) + r'",\s*\d+\)', source)
        if module == "filesystem" and not rust:
            rust = re.search(r'\("' + re.escape(function) + r'",\s*(?:true|false)\)', source)
        if not rust and not lua:
            raise ValueError(f"Palette function is not registered: {signature}")
        counters["API palette entries"] += 1
    counters["workspace crates"] = len(names)
    counters["Rust source files"] = sum(p.suffix == ".rs" for p in paths)
    counters["Rust test functions (not executed)"] = sum(len(re.findall(r'#\[test\]', p.read_text())) for p in paths if p.suffix == ".rs")
    return counters


def main() -> int:
    try:
        for name, count in audit(Path(__file__).resolve().parents[1]).items():
            print(f"{name}: {count}")
    except (OSError, ValueError, KeyError, TypeError, zlib.error, wave.Error) as error:
        print(f"Source consistency check failed: {error}", file=sys.stderr)
        return 1
    print("Source consistency checks passed. Cargo, GUI and engine execution are separate checks.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
