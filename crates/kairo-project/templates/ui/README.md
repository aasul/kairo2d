# Kairo UI workshop

Run from the repository root:

```sh
cargo run -p kairo-cli -- run examples/ui
```

Left: retained game UI implemented in Lua. Native floating window: queued egui debug controls with one-frame response latency. Mouse clicks change the score and progress bar. Escape exits.

The 3.3 source snapshot has not been compiled or graphically tested. See the root VALIDATION.md.
