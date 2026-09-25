-- Edit this value on the host and save while a trusted tester is connected.
local speed = 220
local player = {x = 440, y = 250, distance = 0}
function game.update(dt)
    local dx, dy = 0, 0
    if keyboard.isDown("a") or keyboard.isDown("left") then dx = -1 end
    if keyboard.isDown("d") or keyboard.isDown("right") then dx = dx + 1 end
    if keyboard.isDown("w") or keyboard.isDown("up") then dy = -1 end
    if keyboard.isDown("s") or keyboard.isDown("down") then dy = dy + 1 end
    player.x = math.max(0, math.min(936, player.x + dx * speed * dt))
    player.y = math.max(120, math.min(516, player.y + dy * speed * dt))
    player.distance = player.distance + math.sqrt(dx * dx + dy * dy) * speed * dt
end
function game.draw()
    graphics.clear(0.04, 0.07, 0.10)
    graphics.print("KAIRO LINK - EXPLICIT STATE HANDOFF", 24, 24, 2)
    graphics.print("WASD / arrows | host: edit speed and save", 24, 62, 2)
    graphics.print(string.format("Speed %d  |  Distance %.0f", speed, player.distance), 24, 96, 1)
    graphics.setColor(0.4, 0.83, 0.71)
    graphics.rectangle("fill", player.x, player.y, 24, 24)
end
function game.saveState() return player end
function game.restoreState(saved)
    assert(type(saved) == "table" and type(saved.x) == "number" and type(saved.y) == "number")
    player = saved
end
function game.keyPressed(key) if key == "escape" then window.close() end end
