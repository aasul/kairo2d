# Game UI and developer UI

## Retained game UI (`ui`)

`ui` is a shipped Lua module using Kairo's drawing/input APIs. Create panels once,
then update widget state. The runtime updates pointer events before game.update and
draws UI after game.draw with an identity camera/viewport.

```lua
local label
function game.load()
    local panel = ui.panel({x=20,y=20,width=260,height=150,padding=12,gap=8})
    label = panel:add(ui.label("Ready"))
    panel:add(ui.button("Start", function() label:setText("Started") end))
    panel:add(ui.progress(0.5, {height=16}))
end
```

Constructors: `panel(options)`, `label(text[, options])`,
`button(text, onClick[, options])`, `image(texture[, options])`,
`progress(value[, options])`. Panel options include x/y, width/height, padding, gap,
layout (`vertical`/`horizontal`), anchor (`top_left`, `top_right`, `bottom_left`,
`bottom_right`, `center`), and RGBA background. Labels/buttons accept height/width,
text color and bitmap `text_scale`. Use `ui.setFont(font)` for cached font text.

Methods: `panel:add(child)`, `widget:setText(text)`, `setValue(0..1)`,
`setVisible(bool)`, `setEnabled(bool)`. `ui.remove(widget)` detaches it;
`ui.clear()` removes roots and pending pointer state. No node can have two parents
or create a cycle. Limits are 128 roots, 1024 children per panel, depth 16.

A button fires when primary press/release hit the same enabled visible node.
Press/release events within one frame are preserved; window focus loss cancels a
pending press without firing a click. Overlapping children use
reverse order for hit testing; layouts do not clip overflowing children.
`ui.wantsMouse()` lets game code decide whether to ignore pointer controls.
Game input callbacks are not silently suppressed. The framework does not implement
scroll containers, keyboard focus, controller navigation, text entry, accessibility,
CSS, rich text or automatic content sizing. It is suitable for simple HUDs and menus.

## Immediate development UI (`debugui`)

```lua
local speed = 120
function game.debugUI()
    debugui.window("Player", function()
        debugui.text("Runtime controls")
        speed = debugui.slider("Speed", speed, 0, 500)
        local active = debugui.checkbox("Active", true)
        if debugui.button("Reset") then speed = 120 end
    end)
end
```

Available widgets are window, text, button, checkbox and slider. Widgets return the
last native response; because Lua commands are queued then rendered, responses
arrive on the **next** debug-UI callback. Keep stable window titles/widget order and
labels for stable IDs. Windows must be top-level in game.debugUI, not nested or
called from update/load. Limits: 16 windows, 128 widgets each, 128-byte titles,
512-byte labels and 4096-byte text.

The Rust egui painter receives pointer events and renders at window resolution,
after Micro quantization. `debugui.wantsMouse()` reports the previous egui pointer
capture decision. This small bridge does not support text fields, clipboard,
keyboard navigation, arbitrary native egui handles or editor-only docking.

Both systems have actual implementations. UI rendering and GUI interaction remain
unverified until the native workspace can build and run.
