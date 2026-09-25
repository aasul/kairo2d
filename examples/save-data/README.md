# Save data lab

Run from the repository root:

```sh
cargo run -p kairo-cli -- run examples/save-data
```

Uses a stable [save] identity and the per-user data directory, not the project folder. Writes happen on an explicit input action, never while loading a replacement VM. Space increments; S saves; L loads; D deletes.

The 3.3 source snapshot has not been compiled or graphically tested. See the root VALIDATION.md.
