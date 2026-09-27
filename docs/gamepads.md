# Gamepad input

```lua
local ids = gamepad.connected()
if ids[1] then
    local id = ids[1]
    local x, y = gamepad.axis(id, "left_stick")
    local trigger = gamepad.axis(id, "right_trigger")
    if gamepad.isDown(id, "a") then end
end
```

Do not assume controller 1 exists. `connected()` is a sorted array of current
one-based IDs; hotplugging can reuse an ID later. `name(id)` returns a backend name
or nil. A missing controller polls false/zero. Invalid axis/button names are errors.

Axes: `left_stick`, `right_stick` return x,y (positive Y down); `left_trigger` and
`right_trigger` return value,0. Sticks use a 0.15 radial deadzone with rescaling.
Trigger values are provided by gilrs, normally 0..1; backend mapping quality varies.

Buttons: `a`, `b`, `x`, `y`, `start`, `back`, `guide`, `left_shoulder`,
`right_shoulder`, `left_trigger`, `right_trigger`, `left_stick`, `right_stick`,
`up`, `down`, `left`, `right`. Names follow conventional face-button positions,
not the printed labels on every controller.

```lua
function game.gamepadConnected(id, name) end
function game.gamepadDisconnected(id) end
function game.gamepadPressed(id, button) end
function game.gamepadReleased(id, button) end
```

The backend polls before update and delivers queued events against the current
polled state. Multiple rapid changes in one poll may already have reached their
final value when callbacks run. No rumble, remapping editor or per-controller
player assignment is exposed. Headless sessions contain no physical controllers.

Native access uses gilrs. Linux builds require pkg-config/libudev development
files; device permissions still apply. No controller hardware was available for
acceptance testing. The example includes a keyboard fallback.
