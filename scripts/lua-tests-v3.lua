-- Standalone checks exercise actual built-in Lua, with explicit host/renderer doubles.
local rendered, mx, my = {}, 0, 0
window = {getSize = function() return 960, 540 end, close = function() end}
mouse = {position = function() return mx, my end}
graphics = {
    rectangle = function(mode, x, y, w, h)
        assert(mode == "fill" or mode == "line")
        assert(w >= 0 and h >= 0 and x == x and y == y)
        rendered[#rendered + 1] = {x, y, w, h}
    end,
    setColor = function() end, print = function() end, clear = function() end,
    resetCamera = function() end, resetViewport = function() end, resetScissor = function() end,
    draw = function() end
}
local ui = dofile(SOURCE_ROOT .. "/crates/kairo-lua/src/builtin/ui.lua")
local clicks = 0
local root = ui.panel({x = 10, y = 20, width = 200, height = 120})
local button = root:add(ui.button("Increment", function() clicks = clicks + 1 end))
ui._pointer(true, 30, 40, 1)
ui._pointer(false, 30, 40, 1)
ui._update()
assert(clicks == 1, "press and release in the same frame must click")
ui._pointer(true, 30, 40, 1)
ui._pointer(false, 800, 500, 1)
ui._update()
assert(clicks == 1, "release outside must not click")
ui._pointer(true, 30, 40, 1); ui._update()
ui._cancelPointer()
ui._pointer(false, 30, 40, 1); ui._update()
assert(clicks == 1, "focus loss must cancel without a click")
button:setEnabled(false)
ui._pointer(true, 30, 40, 1); ui._pointer(false, 30, 40, 1); ui._update()
assert(clicks == 1, "disabled button accepted input")
assert(not pcall(function() root:add(root) end), "UI parent cycle accepted")
ui._draw(); assert(#rendered >= 2)
ui.remove(root); rendered = {}; ui._draw(); assert(#rendered == 0)
root = ui.panel({anchor = "bottom_right", x = 10, y = 10, width = 100, height = 80})
ui._update(); assert(root.x == 850 and root.y == 450)
mx, my = 875, 470; assert(ui.wantsMouse())
ui.clear(); assert(not ui.wantsMouse())
print("Built-in Lua UI tests: click ordering, cancellation, disabled buttons, cycles, layout, removal passed")

local saved, restore, cleared = nil, nil, 0
local host = {
    store = function(value) saved = value end,
    restore = function() local value = restore; restore = nil; return value end,
    clear = function() cleared = cleared + 1 end,
    enable = function() end, stats = function() return {} end,
    request = function() end, due = function() return false end
}
local replay = assert(loadfile(SOURCE_ROOT .. "/crates/kairo-lua/src/builtin/replay.lua"))(host)
local player = {x = 4, inventory = {gold = 9}, flags = {[true] = "ready", [3] = false}}
local inventory = player.inventory
replay.register("player", player); assert(cleared == 1)
replay.capture(); assert(saved.kind == "table")
player.x = 999; player.inventory.gold = 0; player.extra = "erase"
restore = saved; replay._before()
assert(player.x == 4 and player.inventory == inventory and inventory.gold == 9 and player.extra == nil)
assert(player.flags[true] == "ready" and player.flags[3] == false)
for _, value in ipairs({function() end, coroutine.create(function() end), setmetatable({}, {})}) do
    assert(not pcall(replay._encode, value), "unsupported replay value accepted")
end
local cycle = {}; cycle.self = cycle
assert(not pcall(replay._encode, cycle))
local shared = {}; assert(not pcall(replay._encode, {a = shared, b = shared}))
assert(not pcall(replay._encode, math.huge)); assert(not pcall(replay._encode, 0 / 0))
local deep = {}; local cursor = deep
for _ = 1, 30 do cursor.next = {}; cursor = cursor.next end
assert(not pcall(replay._encode, deep))
local scalar = {false, "hello", 12.5}
for _, value in ipairs(scalar) do assert(replay._decode(replay._encode(value)) == value) end
print("Built-in Lua Replay tests: in-place restoration, key types, scalar roundtrip, alias/cycle/type/depth rejection passed")

-- Demo behavior checks below deliberately do NOT substitute for Rust integration tests.
local keys = {}
keyboard = {isDown = function(key) return keys[key] or false end}
local closed = false
window.close = function() closed = true end
local function open_demo(name)
    game = {}
    dofile(SOURCE_ROOT .. "/examples/" .. name .. "/main.lua")
    if game.load then game.load() end
end
local random_state = 0
random = {
    seed = function(seed) random_state = seed end,
    integer = function(min, max)
        random_state = (random_state * 1664525 + 1013904223) % 4294967296
        return min + random_state % (max - min + 1)
    end
}
open_demo("micro")
keys.left = true
for _ = 1, 1800 do game.update(1 / 60); game.draw() end
keys = {}; game.keyPressed("r"); game.update(1 / 60); game.draw()
print("Micro Lua game with RNG/graphics/input doubles: 1800 ticks, bounds, game-over and restart passed")
open_demo("link")
keys.d = true; game.update(0.1); keys = {}
local previous = game.saveState(); assert(previous.x > 440 and previous.distance > 0)
open_demo("link"); game.restoreState(previous)
assert(game.saveState().x == previous.x and game.saveState().distance == previous.distance)
game.draw()
assert(not pcall(game.restoreState, "invalid"))
print("Link demo Lua state hooks: explicit state roundtrip and invalid-state rejection passed")

_G.ui = ui
open_demo("ui")
for _ = 1, 60 do game.update(1 / 60); game.draw(); ui._update(); ui._draw() end
ui._pointer(true, 60, 130, 1); ui._pointer(false, 60, 130, 1); ui._update(); ui._draw()
debugui = {
    window = function(_, fn) fn() end, text = function() end,
    slider = function(_, value) return value end, checkbox = function(_, value) return value end,
    button = function() return false end
}
game.debugUI()
print("UI demo with graphics/debugui doubles: construction, update/draw and native-widget callback passed")

gamepad = {connected = function() return {} end, axis = function() return 0, 0 end}
open_demo("gamepad")
keys.s = true
for _ = 1, 60 do game.update(1 / 60); game.draw() end
keys = {}; game.gamepadPressed(1, "a"); game.gamepadReleased(1, "a")
filesystem = {exists = function() return false end}
open_demo("fonts"); game.draw()
print("Gamepad demo keyboard fallback and font demo no-font fallback passed with explicit boundary doubles")
