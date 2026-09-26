local boxTexture
local boxes = {}

local function spawn(x, y)
    if #boxes >= 64 then return end
    local body = physics.newRectangle("dynamic", x, y, 32, 32)
    body:setRotation((#boxes % 5) * 0.15)
    boxes[#boxes + 1] = body
end

function game.load()
    boxTexture = graphics.loadTexture("assets/box.png")
    physics.newRectangle("static", 480, 500, 920, 24)
    physics.newRectangle("static", 20, 270, 24, 500)
    physics.newRectangle("static", 940, 270, 24, 500)
    for i = 1, 8 do spawn(300 + i * 38, 80 - i * 35) end
end

function game.draw()
    graphics.clear(0.06, 0.08, 0.11)
    graphics.setColor(0.25, 0.35, 0.43)
    graphics.rectangle("fill", 20, 488, 920, 24)
    graphics.rectangle("fill", 8, 20, 24, 480)
    graphics.rectangle("fill", 928, 20, 24, 480)
    graphics.setColor(1, 1, 1)
    for _, body in ipairs(boxes) do
        local x, y = body:getPosition()
        graphics.draw(boxTexture, {
            x = x, y = y, rotation = body:getRotation(),
            origin_x = 16, origin_y = 16
        })
    end
    graphics.print("Click: drop a box | Space: launch boxes | F5: reset", 48, 28)
end

function game.mousePressed(x, y, button)
    if button == 1 then spawn(x, y) end
end

function game.keyPressed(key)
    if key == "escape" then window.close() end
    if key == "space" then
        for _, body in ipairs(boxes) do body:setVelocity(0, -450) end
    end
end
