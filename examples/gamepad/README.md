# Controller lab

Run from the repository root:

```sh
cargo run -p kairo-cli -- run examples/gamepad
```

Left stick moves the square; right trigger fills the bar; button callbacks update the label. Gamepad hardware behavior is unverified in this source snapshot. Keyboard fallback works without a controller.

The 3.3 source snapshot has not been compiled or graphically tested. See the root VALIDATION.md.
