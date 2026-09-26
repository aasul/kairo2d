# Kairo Micro - Starcatcher

Run from the repository root:

```sh
cargo run -p kairo-cli -- run examples/micro
```

Catch falling stars with arrows or A/D. R restarts. Internal canvas is 320x180; presentation is integer-scaled and quantized to the original Kairo16 palette. Resizing smaller than the canvas permits fractional downscaling.
