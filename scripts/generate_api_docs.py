#!/usr/bin/env python3
"""Generate the bundled searchable reference from the editor's API metadata."""
import argparse
import json
import re
from pathlib import Path

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--check", action="store_true")
args = parser.parse_args()
entries = json.loads((root / "docs/lua-api.json").read_text())
lines = ["# API signatures and snippets", "", "Generated from `docs/lua-api.json`. The editor uses the same metadata offline.",
         "Methods headed Animator, Emitter and TileMap refer to returned objects, not global modules.",
         "Snippets illustrate signatures; variables such as player/map/sparks must exist in your game.", ""]
for entry in entries:
    lines.extend(["## `" + entry["signature"] + "`", "", entry["description"], "", "```lua", entry["snippet"], "```", ""])
text = "\n".join(lines)
path = root / "docs/api-reference.md"
if args.check:
    if not path.exists() or path.read_text() != text: raise SystemExit("API reference is stale: run python scripts/generate_api_docs.py")
else:
    path.write_text(text, encoding="utf-8")

# Keep LuaLS completion and signature help for development debug APIs tied to
# the same editor metadata. These declarations have no runtime effect.
stubs = ["---@meta", "", "profiler = {}", "debug = {}", ""]
for entry in entries:
    match = re.fullmatch(r"(profiler|debug)\.([A-Za-z_]\w*)\(([^)]*)\)", entry["signature"])
    if not match:
        continue
    module, method, parameters = match.groups()
    stubs.append("---" + entry["description"])
    parameters = [parameter.strip() for parameter in parameters.split(",") if parameter.strip()]
    for parameter in parameters:
        stubs.append(f"---@param {parameter} {'boolean' if parameter == 'enabled' else 'any'}")
    if module == "profiler" and method == "stats":
        stubs.append("---@return table")
    stubs.extend([f"function {module}.{method}({', '.join(parameters)}) end", ""])
stub_text = "\n".join(stubs)
stub_path = root / "docs/luals/debug-profiler.lua"
if args.check:
    if not stub_path.exists() or stub_path.read_text() != stub_text:
        raise SystemExit("LuaLS definitions are stale: run python scripts/generate_api_docs.py")
else:
    stub_path.parent.mkdir(parents=True, exist_ok=True)
    stub_path.write_text(stub_text, encoding="utf-8")
# Scene-node methods use the same canonical signatures as editor completion.
node_stubs = ["---@meta", "", "---@class KairoNode", "local KairoNode = {}", ""]
for entry in entries:
    match = re.fullmatch(r"node:([A-Za-z_]\w*)\(([^)]*)\)", entry["signature"])
    if not match:
        continue
    method, parameters = match.groups()
    parameters = [parameter.strip() for parameter in parameters.split(",") if parameter.strip()]
    node_stubs.append("---" + entry["description"])
    for parameter in parameters:
        node_stubs.append(f"---@param {parameter} any")
    node_stubs.extend([f"function KairoNode:{method}({', '.join(parameters)}) end", ""])
node_text = "\n".join(node_stubs)
node_path = root / "docs/luals/scene-node.lua"
if args.check:
    if not node_path.exists() or node_path.read_text() != node_text:
        raise SystemExit("Scene-node LuaLS definitions are stale: run python scripts/generate_api_docs.py")
else:
    node_path.parent.mkdir(parents=True, exist_ok=True)
    node_path.write_text(node_text, encoding="utf-8")
print(f"API reference and LuaLS definitions: {len(entries)} entries synchronized")
