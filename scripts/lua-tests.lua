-- Optional standalone Lua tests. Engine boundaries below are explicit test doubles.
local function near(a, b)
    assert(math.abs(a - b) < 0.00001, string.format("expected %.6f, got %.6f", b, a))
end

local collision = dofile(SOURCE_ROOT .. "/examples/breakout/collision.lua")
local box = { x = 10, y = 10, w = 20, h = 20 }
assert(collision.circleBox(0, 0, 2, box) == nil)
assert(collision.circleBox(8, 20, 2, box) == nil)
local nx, ny, depth = collision.circleBox(9, 20, 2, box)
near(nx, -1); near(ny, 0); near(depth, 1)
nx, ny, depth = collision.circleBox(20, 11, 2, box)
near(nx, 0); near(ny, -1); near(depth, 3)
nx, ny, depth = collision.circleBox(31, 31, 2, box)
near(nx, 1 / math.sqrt(2)); near(ny, 1 / math.sqrt(2)); near(depth, 2 - math.sqrt(2))
local vx, vy = collision.reflect(10, 5, -1, 0)
near(vx, -10); near(vy, 5)
vx, vy = collision.reflect(-10, 5, -1, 0)
near(vx, -10); near(vy, 5)
print("Pure Lua collision tests: separation, tangency, face, interior, corner, reflection passed")

local keys = {}
keyboard = { isDown = function(key) return keys[key] or false end }
window = { getSize = function() return 960, 540 end }
local Player = dofile(SOURCE_ROOT .. "/examples/movement/player.lua")
local player = Player.new()
keys.d = true
player:update(0.25)
near(player.x, 160); near(player.y, 180)
keys.w = true
player:update(0.25)
near(player.x, 160 + 60 / math.sqrt(2)); near(player.y, 180 - 60 / math.sqrt(2))
keys.a, keys.s = true, true
local previous_x, previous_y = player.x, player.y
player:update(0.25)
near(player.x, previous_x); near(player.y, previous_y)
keys = { d = true }
player:update(100)
near(player.x, 896)
print("Movement Lua tests with injected input/window: speed, diagonal normalization, opposing keys, bounds passed")

local text, draw, rectangles = {}, nil, {}
graphics = {
    loadTexture = function(path) assert(path == "assets/ball.png"); return { test_texture = true } end,
    clear = function() text, draw, rectangles = {}, nil, {} end,
    setColor = function() end,
    rectangle = function(_, x, y, w, h) rectangles[#rectangles + 1] = { x = x, y = y, w = w, h = h } end,
    print = function(value) text[value] = true end,
    draw = function(texture, x, y) assert(texture.test_texture); draw = { x = x, y = y } end,
}
audio = { isAvailable = function() return false end }
local closed = false
window.close = function() closed = true end
package.path = SOURCE_ROOT .. "/examples/breakout/?.lua;" .. package.path
game = {}
keys = {}
dofile(SOURCE_ROOT .. "/examples/breakout/main.lua")
game.load()
game.draw()
assert(text["SPACE TO LAUNCH"] and text["LIVES 3"] and text["SCORE 0"])
assert(#rectangles == 48, "expected separator, 45 bricks, paddle and state overlay")
local initial_x, initial_y = draw.x, draw.y
keys.d = true
game.update(0.1); game.draw()
near(draw.x, initial_x + 56); near(draw.y, initial_y)
keys = {}
game.keyPressed("space")
game.update(0.1); game.draw()
assert(draw.y < initial_y and not text["SPACE TO LAUNCH"])
game.keyPressed("p")
local paused_x, paused_y = draw.x, draw.y
game.update(1); game.draw()
assert(text["PAUSED - P TO RESUME"])
near(draw.x, paused_x); near(draw.y, paused_y)
game.keyPressed("p")
for _ = 1, 600 do game.update(1 / 60); game.draw() end
game.keyPressed("r"); game.draw()
assert(text["SPACE TO LAUNCH"] and text["LIVES 3"] and text["SCORE 0"])
near(draw.x, initial_x); near(draw.y, initial_y)
game.keyPressed("escape"); assert(closed)
print("Breakout Lua tests with graphics/audio/window doubles: startup, movement, launch, pause, 600 updates, restart, quit passed")
