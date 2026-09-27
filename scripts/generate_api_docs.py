#!/usr/bin/env python3
"""Generate the bundled searchable reference from the editor's API metadata."""
import argparse
import json
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
print(f"API reference: {len(entries)} entries synchronized")
