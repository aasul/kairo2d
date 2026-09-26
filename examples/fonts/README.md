# Font atlas lab

Run from the repository root:

```sh
cargo run -p kairo-cli -- run examples/fonts
```

No font files are distributed. Supply a font you are allowed to use as assets/font.ttf or assets/font.otf. TrueType-outline OpenType files are supported; color fonts, shaping/bidi, fallback and CFF-only support are not promised. Without a supplied font the example uses bitmap text.
