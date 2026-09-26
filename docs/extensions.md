# Rust extensions (experimental source API)

Lua remains the game language. A Rust embedding application can add a statically
compiled `kairo_core::extensions::NativeExtension` to a `GameSession`. The trait
provides `name`, `update(dt, &InputState)` and `draw(&mut Canvas)`; `Canvas` offers
validated rectangle and sprite commands using Kairo handles. It never exposes a
raw graphics device. Errors carry extension/callback context.

```sh
cargo run -p kairo-lua --example native_extension
```

The included example embeds a real session headlessly and adds a moving marker.
Native update runs before Lua update; native drawing follows Lua draw and precedes
retained game UI. Registration is capped at 64 uniquely named extensions. Extensions
stay attached across session reload through shared native ownership.

This is a **source-level extension point, not a stable binary ABI**. Rebuild extensions
with the engine. The host uses trusted Rust code with normal process privileges;
there is no dynamic DLL loader, plugin manifest scan, unload/hot-reload protocol,
C/C++ ABI binding generator or C# runtime. Native state is not automatically captured
by Replay. There is no WASM runtime or fake WASM command in this version.

The separation between input/commands and Lua is the future binding seam. Add native
services only when a real game needs them; do not expose backend types merely to
support a speculative language binding.
