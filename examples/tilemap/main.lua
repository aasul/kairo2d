local map
local camera = {x = 0, y = 0}
function game.load()
    map = tilemap.load("assets/world.tmj")
end
function game.update(dt)
    local dx, dy = 0, 0
    if keyboard.isDown("a") or keyboard.isDown("left") then dx = -1 end
    if keyboard.isDown("d") or keyboard.isDown("right") then dx = dx + 1 end
    if keyboard.isDown("w") or keyboard.isDown("up") then dy = -1 end
    if keyboard.isDown("s") or keyboard.isDown("down") then dy = dy + 1 end
    camera.x = math.max(0, math.min(256, camera.x + dx * 170 * dt))
    camera.y = math.max(0, math.min(204, camera.y + dy * 170 * dt))
end
function game.draw()
    graphics.clear(0.035, 0.05, 0.07)
    graphics.setCamera(camera.x, camera.y, 2)
    map:draw()
    for _, object in ipairs(map:getObjects()) do
        graphics.setColor(1, 0.85, 0.4)
        graphics.rectangle("line", object.x, object.y, object.width, object.height)
    end
    graphics.resetCamera()
    graphics.setColor(0.04, 0.06, 0.1, 0.94)
    graphics.rectangle("fill", 0, 0, 960, 64)
    graphics.setColor(1, 1, 1)
    graphics.print("WASD / arrows: scroll | finite Tiled JSON + .tsj", 24, 22, 2)
end
function game.keyPressed(key)
    if key == "escape" then window.close() end
end
