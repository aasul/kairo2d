# Kairo Replay lab

Run from the repository root:

```sh
cargo run -p kairo-cli -- run examples/replay
```

R restores a prior registered state and pauses; Space resumes from that point and discards the future branch; N advances one simulation tick. RNG and trail are restored. Use the editor Replay panel while running. This is explicit snapshot restoration, not arbitrary-memory or frame-perfect deterministic rewind.
