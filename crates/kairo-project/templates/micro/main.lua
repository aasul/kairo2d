local ship = {x = 156, score = 0, lives = 3}
local stars = {}
local timer = 0
local function reset()
    ship.x, ship.score, ship.lives = 156, 0, 3
    stars, timer = {}, 0
    random.seed(9123)
end
function game.load() reset() end
function game.update(dt)
    if ship.lives <= 0 then return end
    local direction = 0
    if keyboard.isDown("left") or keyboard.isDown("a") then direction = -1 end
    if keyboard.isDown("right") or keyboard.isDown("d") then direction = direction + 1 end
    ship.x = math.max(4, math.min(306, ship.x + direction * 145 * dt))
    timer = timer + dt
    if timer >= 0.45 then
        timer = timer - 0.45
        stars[#stars + 1] = {x = random.integer(4, 307), y = 18, speed = random.integer(35, 85)}
    end
    for i = #stars, 1, -1 do
        local star = stars[i]
        star.y = star.y + star.speed * dt
        if star.y >= 156 and star.y <= 169 and star.x + 4 >= ship.x and star.x <= ship.x + 10 then
            ship.score = ship.score + 1; table.remove(stars, i)
        elseif star.y > 180 then ship.lives = ship.lives - 1; table.remove(stars, i) end
    end
end
function game.draw()
    graphics.clear(0.04, 0.06, 0.1)
    graphics.setColor(0.32, 0.64, 0.62)
    for i = 1, 20 do
        graphics.rectangle("fill", (i * 71) % 320, (i * 43) % 180, 1, 1)
    end
    graphics.setColor(0.96, 0.77, 0.39)
    for _, star in ipairs(stars) do graphics.rectangle("fill", star.x, star.y, 4, 4) end
    graphics.setColor(0.45, 0.9, 0.78)
    graphics.rectangle("fill", ship.x, 160, 10, 5)
    graphics.rectangle("fill", ship.x + 4, 157, 2, 3)
    graphics.setColor(1, 1, 1)
    graphics.print("STARS " .. ship.score .. "  LIVES " .. math.max(0, ship.lives), 6, 5, 1)
    if ship.lives <= 0 then
        graphics.setColor(0.05, 0.07, 0.12)
        graphics.rectangle("fill", 57, 63, 206, 43)
        graphics.setColor(1, 1, 1)
        graphics.print("GAME OVER - R TO RESTART", 68, 79, 1)
    end
end
function game.keyPressed(key)
    if key == "r" then reset()
    elseif key == "escape" then window.close() end
end
