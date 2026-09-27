local level = scene.new("Arena")
local bodies = {}

for i = 1, 48 do
    local enemy = level:createNode("Node2D", "Enemy" .. i)
    level:attachScript(enemy, "scripts/enemy.lua")
end
scene.switch(level)

function game.load()
    profiler.show(true)
    for i = 1, 12 do
        bodies[i] = physics.newRectangle("dynamic", 100 + i * 55, 100, 24, 24)
    end
    physics.newRectangle("static", 480, 520, 900, 20)
end

function game.draw()
    graphics.clear(0.05, 0.07, 0.1)
    for i = 1, 48 do
        local x = 35 + ((i - 1) % 12) * 74
        local y = 140 + math.floor((i - 1) / 12) * 72
        graphics.setColor(0.25 + i / 120, 0.65, 0.9)
        graphics.rectangle("fill", x, y, 22, 22)
    end
    graphics.setColor(1, 1, 1)
    graphics.print("Run with --profile debug, then open Editor Profiler (F10)", 30, 60)
end
