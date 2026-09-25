local clip
local paused = false
function game.load()
    clip = animation.load("assets/pilot.anim.json")
end
function game.update(dt)
    clip:update(dt)
end
function game.draw()
    graphics.clear(0.055, 0.075, 0.11)
    graphics.print("ANIMATION WORKSHOP", 36, 36, 3)
    graphics.print("Space: pause / resume    R: restart", 36, 84, 2)
    graphics.draw(clip, {x = 280, y = 190, scale_x = 10, scale_y = 10})
    graphics.draw(clip, {x = 640, y = 230, scale_x = -5, scale_y = 5})
    graphics.print("Frame " .. clip:getFrame(), 36, 440, 2)
end
function game.keyPressed(key)
    if key == "space" then
        paused = not paused
        if paused then clip:pause() else clip:resume() end
    elseif key == "r" then clip:restart()
    elseif key == "escape" then window.close() end
end
