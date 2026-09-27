local Player = require("player")
local player
local texture

function game.load()
    player = Player.new()
    texture = graphics.loadTexture("assets/player.png")
end

function game.update(dt)
    player:update(dt)
end

function game.draw()
    graphics.clear(0.04, 0.08, 0.10)
    graphics.print("Move: WASD / arrows | click: teleport | Escape: quit", 32, 32)
    graphics.draw(texture, {
        x = player.x, y = player.y,
        scale_x = 2, scale_y = 2
    })
    local x, y = mouse.position()
    graphics.setColor(0.35, 0.65, 0.7)
    graphics.rectangle("line", x - 8, y - 8, 16, 16)
end

function game.mousePressed(x, y, button)
    if button == 1 then player.x, player.y = x - 32, y - 32 end
end

function game.keyPressed(key)
    if key == "escape" then window.close() end
end
