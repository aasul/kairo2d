local font
local message = "Add your own licensed assets/font.ttf (or font.otf), then restart."
function game.load()
    for _, path in ipairs({"assets/font.ttf", "assets/font.otf"}) do
        if filesystem.exists(path) then
            font = graphics.loadFont(path, 28)
            message = "Glyphs are cached in a texture atlas."
            break
        end
    end
end
function game.draw()
    graphics.clear(0.045, 0.065, 0.1)
    graphics.print("FONT ATLAS LAB", 32, 32, 3)
    graphics.print(message, 32, 88, 1)
    if font then
        local text = "Hello Kairo!\nAVATAR 0123456789\nThe quick brown fox."
        local width, height = graphics.measureText(font, text)
        graphics.setColor(0.3, 0.65, 0.7)
        graphics.rectangle("line", 32, 150, width, height)
        graphics.setColor(1, 1, 1)
        graphics.print(font, text, 32, 150)
    else
        graphics.print("The built-in bitmap font needs no external font file.", 32, 160, 2)
    end
end
function game.keyPressed(key) if key == "escape" then window.close() end end
