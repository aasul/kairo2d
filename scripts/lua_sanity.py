#!/usr/bin/env python3
"""Compile Lua with native Lua 5.4 and test pure game logic. Does not execute Rust or the engine."""
from __future__ import annotations

import argparse
import ctypes
import ctypes.util
import json
from pathlib import Path
import sys
import tomllib


class Lua54:
    def __init__(self, library: str | None = None):
        name = library or ctypes.util.find_library("lua5.4") or ctypes.util.find_library("lua54")
        if not name:
            raise RuntimeError("A native Lua 5.4 shared library is required for this optional check")
        self.lib = lib = ctypes.CDLL(name)
        lib.luaL_newstate.restype = ctypes.c_void_p
        lib.lua_close.argtypes = [ctypes.c_void_p]
        lib.luaL_openlibs.argtypes = [ctypes.c_void_p]
        lib.luaL_loadbufferx.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_size_t, ctypes.c_char_p, ctypes.c_char_p]
        lib.luaL_loadbufferx.restype = ctypes.c_int
        lib.lua_pcallk.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_ssize_t, ctypes.c_void_p]
        lib.lua_pcallk.restype = ctypes.c_int
        lib.lua_tolstring.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.POINTER(ctypes.c_size_t)]
        lib.lua_tolstring.restype = ctypes.c_void_p
        lib.lua_settop.argtypes = [ctypes.c_void_p, ctypes.c_int]
        lib.lua_pushlstring.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_size_t]
        lib.lua_pushlstring.restype = ctypes.c_void_p
        lib.lua_setglobal.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
        self.state = lib.luaL_newstate()
        if not self.state:
            raise RuntimeError("Unable to allocate a Lua state")
        lib.luaL_openlibs(self.state)
        self.run(b'assert(_VERSION == "Lua 5.4", "Lua 5.4 is required")', "version-check")

    def error(self) -> str:
        length = ctypes.c_size_t()
        pointer = self.lib.lua_tolstring(self.state, -1, ctypes.byref(length))
        return ctypes.string_at(pointer, length.value).decode("utf-8", errors="replace") if pointer else "Unknown Lua error"

    def compile(self, source: bytes, name: str, execute: bool = False) -> None:
        code = self.lib.luaL_loadbufferx(self.state, source, len(source), ("@" + name).encode(), b"t")
        if code == 0 and execute:
            code = self.lib.lua_pcallk(self.state, 0, 0, 0, 0, None)
        error = self.error() if code else None
        self.lib.lua_settop(self.state, 0)
        if error:
            raise RuntimeError(error)

    def run(self, source: bytes, name: str) -> None:
        self.compile(source, name, execute=True)

    def set_string(self, name: str, value: str) -> None:
        encoded = value.encode("utf-8")
        self.lib.lua_pushlstring(self.state, encoded, len(encoded))
        self.lib.lua_setglobal(self.state, name.encode("ascii"))

    def close(self) -> None:
        if self.state:
            self.lib.lua_close(self.state)
            self.state = None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--library", help="Explicit path/name of the Lua 5.4 shared library")
    args = parser.parse_args()
    lua = None
    try:
        root = Path(__file__).resolve().parents[1]
        lua = Lua54(args.library)
        files = sorted(p for p in root.rglob("*.lua") if not any(part in {"target", "dist", ".git"} for part in p.relative_to(root).parts))
        for path in files:
            lua.compile(path.read_bytes(), path.relative_to(root).as_posix())
        print(f"Native Lua 5.4 syntax: {len(files)} files passed", flush=True)
        api = json.loads((root / "docs/lua-api.json").read_text())
        for index, entry in enumerate(api):
            lua.compile(entry["snippet"].encode(), f"api-palette-{index}")
        print(f"Native Lua 5.4 syntax: {len(api)} API snippets passed", flush=True)
        lua.set_string("SOURCE_ROOT", root.as_posix())
        lua.run((root / "scripts/lua-tests.lua").read_bytes(), "scripts/lua-tests.lua")
        lua.run((root / "scripts/lua-tests-v3.lua").read_bytes(), "scripts/lua-tests-v3.lua")
        lua.run((root / "scripts/lua-tests-v34.lua").read_bytes(), "scripts/lua-tests-v34.lua")
        def literal(value):
            if isinstance(value, dict):
                return "{" + ",".join("[" + literal(k) + "]=" + literal(v) for k,v in value.items()) + "}"
            if isinstance(value, list):
                return "{" + ",".join(literal(v) for v in value) + "}"
            if value is None: return "nil"
            if isinstance(value, bool): return "true" if value else "false"
            if isinstance(value, str):
                return '"' + "".join("\\" + str(byte).zfill(3) for byte in value.encode("utf-8")) + '"'
            return repr(value)
        fixtures = {}
        for path in (root / "examples").rglob("*"):
            if not path.is_file(): continue
            name = path.relative_to(root).as_posix()
            if path.suffix == ".toml": fixtures[name] = tomllib.loads(path.read_text())
            elif path.suffix in {".json", ".tmj", ".tsj"}: fixtures[name] = json.loads(path.read_text())
            elif path.suffix in {".wav", ".png"}: fixtures[name] = True
        lua.set_string("DEMO_FIXTURES", "return " + literal(fixtures))
        lua.run((root / "scripts/lua-demo-tests-v34.lua").read_bytes(), "scripts/lua-demo-tests-v34.lua")
    except (OSError, RuntimeError, ValueError) as error:
        print(f"Lua-only validation failed: {error}", file=sys.stderr)
        return 1
    finally:
        if lua:
            lua.close()
    print("Lua-only checks passed. Test doubles above are not Rust engine or GUI validation.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
