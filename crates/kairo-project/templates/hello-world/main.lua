function game.draw()
    graphics.clear(0.05, 0.07, 0.10)

    graphics.setColor(0.20, 0.85, 0.75)
    graphics.rectangle("fill", 64, 130, 180, 180)

    graphics.setColor(0.95, 0.65, 0.30)
    graphics.rectangle("line", 284, 130, 180, 180, 4)

    graphics.setColor(0.45, 0.55, 0.95, 0.6)
    graphics.rectangle("fill", 190, 230, 180, 180)

    graphics.setColor(1, 1, 1)
    graphics.print("Hello, Kairo2D!", 64, 48, 3)
    graphics.print("Edit this file to reload. Escape closes the window.", 64, 462)
end

function game.keyPressed(key)
    if key == "escape" then window.close() end
end
