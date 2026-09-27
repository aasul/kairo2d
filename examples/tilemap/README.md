# Tiled map explorer

Run from the repository root:

```sh
cargo run -p kairo-cli -- run examples/tilemap
```

Finite orthogonal Tiled JSON; the tileset is external JSON. Camera-based tile culling and object rectangles are demonstrated. WASD/arrows pan. Unsupported diagonal flips/infinite/group layers are rejected explicitly.
