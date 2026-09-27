local data = {score = 0, volume = 0.8}
local message = "Space: increment | S: save | L: load | D: delete"
function game.load()
    local saved = save.read("progress.json")
    if saved then data = saved end
end
function game.keyPressed(key)
    if key == "space" then data.score = data.score + 1
    elseif key == "s" then save.write("progress.json", data); message = "Saved in the per-user data directory."
    elseif key == "l" then data = save.read("progress.json") or {score = 0, volume = 0.8}; message = "Loaded."
    elseif key == "d" then save.remove("progress.json"); message = "Save deleted."
    elseif key == "escape" then window.close() end
end
function game.draw()
    graphics.clear(0.055, 0.075, 0.10)
    graphics.print("SAVE DATA LAB", 32, 32, 3)
    graphics.print("Score: " .. data.score, 32, 112, 3)
    graphics.print(message, 32, 200, 2)
end
