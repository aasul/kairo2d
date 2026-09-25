# Input actions

Raw keyboard/mouse/gamepad APIs remain available. Named actions are sampled once before each update, after native input events. Poll action edges in `game.update` or a scene update, not inside raw event callbacks.

```toml
# input.toml in the game root
[actions.jump]
keyboard = ["space"]
mouse = [1]
gamepad = ["a"]
player = 1

[axes.move_x]
negative = ["a", "left"]
positive = ["d", "right"]
gamepad_axis = "left_x"
player = 1
dead_zone = 0.18
```

An existing `input.toml` is loaded during session creation. Open **Input Mappings** in the FEATURES bar, or use the command palette, to edit it. Save writes validated TOML and detects external changes. Restart to apply reliably. Structured input editing rewrites the mapping document; it does not preserve its comments or arbitrary extra fields.

`input.bind(name, table)` and `input.bindAxis(name, table)` override definitions in memory. `action`/`held`, `pressed`, `released`, and `axis` reject unknown names. `input.load(path?)` defaults to `input.toml`; `serialize()` returns TOML; `apply(text)` validates before replacing the current mappings. Rebinding resets cached states, so rebind during setup rather than every frame.

Actions combine keyboard, mouse and one selected controller with OR semantics. Short keyboard/mouse/controller taps recorded between samples survive as pressed and released in the same sample. Controller IDs are one-based connection IDs, not permanent player assignments. All listed buttons/keys use existing engine names.

Digital axes cancel opposing directions. Nonzero digital input takes precedence over the analog scalar. Dead zones rescale the remainder to full range. Controller stick polling already has the backend radial dead zone; this action scalar zone is additional. Axis identifiers: `left_x`, `left_y`, `right_x`, `right_y`, `left_trigger`, `right_trigger`. Screen Y points down.

Limits: 128 actions, 64 axes, 16 keys/buttons per binding, 256 KiB mapping text. Mouse bindings are 1..5. Rebinding does not write save data automatically. The editor provides text/button fields, not a press-any-key capture dialog. See [Input Mapping Demo](../examples/input-actions/README.md).
