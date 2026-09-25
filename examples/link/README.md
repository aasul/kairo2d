# Kairo Link live iteration

Run from the repository root:

```sh
cargo run -p kairo-cli -- run examples/link
```

WASD moves. Edit local speed on the host and save. saveState/restoreState explicitly transfer the player table into the new VM; the code-level speed changes. See docs/kairo-link.md for authenticated opt-in connection steps. Copy this example (including its save identity) to the tester first. No shared token is shipped.

The 3.3 source snapshot has not been compiled or graphically tested. See the root VALIDATION.md.
