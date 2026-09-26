# Third-party components

Kairo source and original example PNG/WAV assets are MIT-licensed. Dependencies retain
their own licenses. The following are implementation references, not a substituted
resolved license bill of materials:

| Component | Reference |
| --- | --- |
| mlua / vendored Lua 5.4 | https://docs.rs/mlua/0.10.3/mlua/ and https://www.lua.org/license.html |
| winit / wgpu / egui | https://docs.rs/winit/0.30.9/winit/ , https://docs.rs/wgpu/0.20.1/wgpu/ , https://docs.rs/egui/0.28.1/egui/ |
| egui-wgpu integration | https://docs.rs/egui-wgpu/0.28.1/egui_wgpu/struct.Renderer.html |
| Kira / Rapier / glam | https://docs.rs/kira/0.9.6/kira/ , https://docs.rs/rapier2d/0.22.0/rapier2d/ , https://docs.rs/glam/0.29.3/glam/ |
| fontdue | https://docs.rs/fontdue/0.9.3/fontdue/ |
| gilrs | https://docs.rs/gilrs/0.11.0/gilrs/ |
| Tiled JSON format | https://doc.mapeditor.org/en/stable/reference/json-map-format/ |
| Noise / snow | https://noiseprotocol.org/noise.html and https://docs.rs/snow/0.10.0/snow/ |
| License collection tool | https://github.com/sstadick/cargo-bundle-licenses |
| Rust MSVC CRT selection | https://doc.rust-lang.org/reference/linkage.html#static-and-dynamic-c-runtimes |

Other dependencies include serde, JSON/TOML parsers, image/PNG/JPEG decoders, notify,
font8x8, tempfile, directories, log, error utilities, rfd and their transitives.
Check the actual resolved Cargo.lock and collected license files rather than treating
this list as exhaustive. There is no fabricated dependency-license report in source.
Native packaging requires a generated nonempty dependency-licenses.json; manually
resolve any missing-license warnings before public redistribution.

No font binary has been copied from the authoring environment into this project.
The font demo deliberately accepts a user-supplied licensed font. eframe's built-in
fonts remain part of the dependency's own distribution and license obligations.

The Micro palettes are original project constants. New sprite/tile PNGs and existing
example WAVs are original programmatically authored assets, not downloaded art.
