local arena = scene.load("scenes/arena.scene")
local boss = assert(arena.graph:findPath("Arena/Boss"))
boss:addTag("boss")
boss:exposeInspector("property.attack_delay", {kind="number", writable=true, persist=true, min=0.1, max=5})
boss:exposeInspector("property.health", {kind="number"})
boss:exposeInspector("position", {kind="vector", writable=true, min=0, max=960})

for i = 1, 40 do
    local minion = arena:createNode("Node2D", "Minion" .. i)
    minion.position = {x = 35 + ((i - 1) % 10) * 90, y = 80 + math.floor((i - 1) / 10) * 95}
end
scene.switch(arena)

local elapsed, attacks = 0, 0
function game.update(dt)
    elapsed = elapsed + dt
    local delay = boss:getProperty("attack_delay")
    if elapsed >= delay then
        elapsed = elapsed - delay
        attacks = attacks + 1
    end
end

function game.draw()
    graphics.clear(0.06, 0.08, 0.13)
    graphics.setColor(0.8, 0.2, 0.25)
    graphics.rectangle("fill", boss.position.x - 25, boss.position.y - 25, 50, 50)
    graphics.setColor(1, 1, 1)
    graphics.print("Boss attacks: " .. attacks, 25, 25)
    graphics.print("Attack delay: " .. boss:getProperty("attack_delay"), 25, 48)
    graphics.print("Select Arena/Boss in Editor Live Inspector", 25, 72)
end
