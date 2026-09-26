local collision = require("collision")
local W, H = 960, 540
local paddle = { x = 416, y = 488, w = 128, h = 14 }
local ball = { x = 480, y = 470, radius = 8, vx = 170, vy = -320 }
local bricks, particles = {}, {}
local mode, score, lives = "ready", 0, 3
local ballTexture, bounceSound
local rowColors = {
    {0.94, 0.42, 0.43}, {0.96, 0.62, 0.38}, {0.89, 0.78, 0.40},
    {0.36, 0.77, 0.64}, {0.36, 0.62, 0.94}
}

local function sound()
    if bounceSound and audio.isAvailable() then
        audio.play(bounceSound, { volume = 0.15 })
    end
end

local function attachBall()
    ball.x, ball.y = paddle.x + paddle.w / 2, paddle.y - ball.radius - 2
    ball.vx, ball.vy = 170, -320
end

local function reset()
    bricks, particles = {}, {}
    for row = 1, 5 do
        for column = 1, 9 do
            bricks[#bricks + 1] = {
                x = 48 + (column - 1) * 97, y = 90 + (row - 1) * 30,
                w = 89, h = 22, color = rowColors[row], points = (6 - row) * 10
            }
        end
    end
    paddle.x, score, lives, mode = 416, 0, 3, "ready"
    attachBall()
end

function game.load()
    ballTexture = graphics.loadTexture("assets/ball.png")
    if audio.isAvailable() then bounceSound = audio.load("assets/bounce.wav") end
    reset()
end

local function burst(brick)
    for i = 1, 8 do
        particles[#particles + 1] = {
            x = brick.x + brick.w / 2, y = brick.y + brick.h / 2,
            vx = (i - 4.5) * 45, vy = -100 - (i % 3) * 35,
            life = 0.5, color = brick.color
        }
    end
end

local function stepBall(dt)
    ball.x, ball.y = ball.x + ball.vx * dt, ball.y + ball.vy * dt
    if ball.x < ball.radius then
        ball.x, ball.vx = ball.radius, math.abs(ball.vx)
        sound()
    elseif ball.x > W - ball.radius then
        ball.x, ball.vx = W - ball.radius, -math.abs(ball.vx)
        sound()
    end
    if ball.y < 54 + ball.radius then
        ball.y, ball.vy = 54 + ball.radius, math.abs(ball.vy)
        sound()
    end

    local nx, ny, depth = collision.circleBox(ball.x, ball.y, ball.radius, paddle)
    if nx and ball.vy > 0 then
        ball.x, ball.y = ball.x + nx * depth, ball.y + ny * depth
        local offset = (ball.x - paddle.x - paddle.w / 2) / (paddle.w / 2)
        ball.vx, ball.vy = math.max(-1, math.min(offset, 1)) * 330, -330
        sound()
    end
    for i = #bricks, 1, -1 do
        local brick = bricks[i]
        nx, ny, depth = collision.circleBox(ball.x, ball.y, ball.radius, brick)
        if nx then
            ball.x, ball.y = ball.x + nx * (depth + 0.01), ball.y + ny * (depth + 0.01)
            ball.vx, ball.vy = collision.reflect(ball.vx, ball.vy, nx, ny)
            score = score + brick.points
            burst(brick)
            table.remove(bricks, i)
            sound()
            break
        end
    end
    if #bricks == 0 then mode = "won" end
    if ball.y > H + ball.radius then
        lives = lives - 1
        mode = lives > 0 and "ready" or "lost"
        attachBall()
    end
end

function game.update(dt)
    if mode == "paused" then return end
    for i = #particles, 1, -1 do
        local p = particles[i]
        p.life = p.life - dt
        p.x, p.y, p.vy = p.x + p.vx * dt, p.y + p.vy * dt, p.vy + 350 * dt
        if p.life <= 0 then table.remove(particles, i) end
    end
    if mode == "ready" or mode == "playing" then
        local direction = 0
        if keyboard.isDown("a") or keyboard.isDown("left") then direction = direction - 1 end
        if keyboard.isDown("d") or keyboard.isDown("right") then direction = direction + 1 end
        paddle.x = math.max(12, math.min(W - paddle.w - 12, paddle.x + direction * 560 * dt))
    end
    if mode == "ready" then attachBall() end
    if mode ~= "playing" then return end
    -- Small collision steps keep a fast ball from skipping a thin brick.
    local steps = math.max(1, math.ceil(dt * 240))
    for _ = 1, steps do
        stepBall(dt / steps)
        if mode ~= "playing" then break end
    end
end

function game.keyPressed(key)
    if key == "space" and mode == "ready" then mode = "playing" end
    if key == "p" then
        if mode == "playing" then mode = "paused"
        elseif mode == "paused" then mode = "playing" end
    end
    if key == "r" then reset() end
    if key == "escape" then window.close() end
end

function game.draw()
    graphics.clear(0.055, 0.065, 0.085)
    graphics.setColor(0.88, 0.91, 0.96)
    graphics.print("KAIRO BREAKOUT", 28, 20, 2)
    graphics.print("SCORE " .. score, 430, 20, 2)
    graphics.print("LIVES " .. lives, 760, 20, 2)
    graphics.setColor(0.17, 0.20, 0.25)
    graphics.rectangle("fill", 0, 52, W, 2)
    for _, brick in ipairs(bricks) do
        graphics.setColor(brick.color[1], brick.color[2], brick.color[3])
        graphics.rectangle("fill", brick.x, brick.y, brick.w, brick.h)
    end
    for _, p in ipairs(particles) do
        graphics.setColor(p.color[1], p.color[2], p.color[3], math.max(0, p.life * 2))
        graphics.rectangle("fill", p.x, p.y, 4, 4)
    end
    graphics.setColor(0.78, 0.84, 0.95)
    graphics.rectangle("fill", paddle.x, paddle.y, paddle.w, paddle.h)
    graphics.setColor(1, 1, 1)
    graphics.draw(ballTexture, ball.x - ball.radius, ball.y - ball.radius)

    local messages = {
        ready = "SPACE TO LAUNCH", paused = "PAUSED - P TO RESUME",
        won = "YOU WIN - R TO RESTART", lost = "GAME OVER - R TO RESTART"
    }
    if messages[mode] then
        local message = messages[mode]
        graphics.setColor(0.04, 0.05, 0.07, 0.92)
        graphics.rectangle("fill", 220, 308, 520, 68)
        graphics.setColor(0.9, 0.94, 1)
        graphics.print(message, (W - #message * 16) / 2, 332, 2)
    end
    graphics.setColor(0.5, 0.57, 0.66)
    graphics.print("A/D OR ARROWS: MOVE   P: PAUSE   R: RESTART   ESC: QUIT", 264, 520, 1)
end
