local points = 0
local speed = 100
local enabled = true
local label, bar
local x = 360
function game.load()
    local panel = ui.panel({x = 24, y = 24, width = 268, height = 260, padding = 14, gap = 10})
    panel:add(ui.label("KAIRO UI WORKSHOP", {height = 28}))
    label = panel:add(ui.label("Score: 0"))
    panel:add(ui.button("Add 10 points", function()
        points = points + 10
        label:setText("Score: " .. points)
        bar:setValue((points % 110) / 100)
    end))
    panel:add(ui.button("Reset", function() points = 0; label:setText("Score: 0"); bar:setValue(0) end))
    bar = panel:add(ui.progress(0, {height = 16}))
    panel:add(ui.label("Escape exits", {height = 20}))
end
function game.update(dt)
    if enabled then x = 330 + ((x - 330 + speed * dt) % 560) end
end
function game.draw()
    graphics.clear(0.04, 0.06, 0.085)
    graphics.setColor(0.4, 0.79, 0.68)
    graphics.rectangle("fill", x, 420, 32, 32)
end
function game.debugUI()
    debugui.window("Native debug controls", function()
        debugui.text("These controls use egui. The game panel uses Lua UI.")
        speed = debugui.slider("Speed", speed, 0, 400)
        enabled = debugui.checkbox("Animate", enabled)
        if debugui.button("Reset position") then x = 360 end
    end)
end
function game.keyPressed(key) if key == "escape" then window.close() end end
