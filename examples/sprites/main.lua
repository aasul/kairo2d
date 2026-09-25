local sheet
local angle = 0

function game.load()
    sheet = graphics.loadTexture("assets/characters.png")
end

function game.update(dt)
    angle = angle + dt * 0.7
end

function game.draw()
    graphics.clear(0.07, 0.08, 0.13)
    graphics.print("Sprites: rotation, origin, scaling and source regions", 32, 32)

    graphics.draw(sheet, 64, 110)
    graphics.draw(sheet, {
        x = 330, y = 280,
        rotation = angle,
        scale_x = 5, scale_y = 5,
        origin_x = 16, origin_y = 16,
        source = {x = 0, y = 0, width = 32, height = 32}
    })

    graphics.setColor(0.65, 0.85, 1)
    graphics.draw(sheet, {
        x = 700, y = 280,
        scale_x = -5, scale_y = 5,
        origin_x = 16, origin_y = 16,
        source = {x = 32, y = 0, width = 32, height = 32}
    })
    graphics.setColor(1, 1, 1)
    graphics.print("One cached texture. Nearest-neighbor filtering.", 32, 470)
end

function game.keyPressed(key)
    if key == "escape" then window.close() end
end
