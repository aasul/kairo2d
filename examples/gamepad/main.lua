local x, y = 450, 280
local lastEvent = "Connect a controller, or use WASD."
function game.update(dt)
    local ids = gamepad.connected()
    local dx, dy = 0, 0
    if ids[1] then dx, dy = gamepad.axis(ids[1], "left_stick") end
    if keyboard.isDown("a") then dx = dx - 1 end
    if keyboard.isDown("d") then dx = dx + 1 end
    if keyboard.isDown("w") then dy = dy - 1 end
    if keyboard.isDown("s") then dy = dy + 1 end
    x, y = math.max(0, math.min(936, x + dx * dt * 220)), math.max(110, math.min(516, y + dy * dt * 220))
end
function game.draw()
    graphics.clear(0.045, 0.07, 0.10)
    graphics.print("CONTROLLER LAB", 24, 20, 3)
    local ids = gamepad.connected()
    graphics.print(ids[1] and gamepad.name(ids[1]) or "No controller (WASD still works)", 24, 58, 2)
    graphics.print(lastEvent, 24, 88, 1)
    graphics.setColor(0.38, 0.82, 0.7)
    graphics.rectangle("fill", x, y, 24, 24)
    if ids[1] then
        local trigger = gamepad.axis(ids[1], "right_trigger")
        graphics.rectangle("fill", 24, 500, 240 * trigger, 12)
    end
end
function game.gamepadPressed(id, button) lastEvent = "Pad " .. id .. " pressed " .. button end
function game.gamepadReleased(id, button) lastEvent = "Pad " .. id .. " released " .. button end
function game.keyPressed(key) if key == "escape" then window.close() end end
