# Kairo Micro

Micro is a profile of the same engine, not a separate language or executable.

In the editor it is labelled **Fantasy Console**. Click that button on the feature
bar (F7), enable the profile, select a preset/palette, then **Save & Run/Restart**.
You can also create the visible Fantasy Console starter in the project hub. No
manual configuration editing is required. See [Feature UI](feature-ui.md).

```toml
[game]
width = 960
height = 540
[micro]
enabled = true
width = 320
height = 180
integer_scaling = true
pixel_snap = true
palette = "kairo16"
```

The game renders to a 320x180 texture in this example. One additional pass scales
that canvas into a centered viewport on the physical window; unused space is black.
Nearest sampling and integer scale factors preserve pixel edges when the window is
large enough. A smaller window uses fractional downscaling rather than cropping.

`window.getSize()`, mouse positions and resize callbacks use canvas pixels in Micro.
`window.getPhysicalSize()` reports actual window pixels; `window.setSize` still
requests a physical window size. Pointer positions in letterbox bars may be outside
the canvas; test bounds rather than clamping every click onto an edge.

Pixel snapping rounds transformed sprite/text vertices to canvas pixels. It does
not quantize physics positions. Palette quantization happens after the scene is
composited; nearest-color selection uses linear RGB distance. `kairo16` is an
original 16-color palette, `grayscale` four neutral colors, and `olive4` four original
olive tones. `none` leaves colors unquantized. The native debug UI is drawn afterward
and is not palette-limited.

The bitmap font and source-rectangle sprites work without new APIs. This profile
has no custom palette-file loader, chiptune emulation, synth API, cartridge format or
hard fantasy-console CPU/resource limits. It is not PICO-8 compatibility. Changing
the profile requires a runtime restart. See the Starcatcher `examples/micro` game.
