local state = {x = 90, y = 270, heading = 1, ticks = 0, trail = {}}
local help = "R: rewind 2 seconds | Space: pause/resume | N: step | Escape: quit"
function game.load()
    random.seed(12345)
    replay.register("runner", state)
    replay.enable(true)
end
function game.update(dt)
    state.x = state.x + state.heading * 150 * dt
    if state.x > 900 then state.x, state.heading = 900, -1 end
    if state.x < 40 then state.x, state.heading = 40, 1 end
    state.ticks = state.ticks + 1
    if state.ticks % 6 == 0 then
        state.y = math.max(170, math.min(420, state.y + random.integer(-12, 12)))
        state.trail[#state.trail + 1] = {x = state.x, y = state.y}
        if #state.trail > 45 then table.remove(state.trail, 1) end
    end
end
function game.draw()
    graphics.clear(0.045, 0.06, 0.09)
    graphics.print("REPLAY LAB", 24, 24, 3)
    graphics.print(help, 24, 72, 1)
    local stats = replay.stats()
    graphics.print(string.format("time %.2f  frame %d  snapshots %d  %s", stats.time, stats.frame,
        #stats.snapshots, stats.paused and "PAUSED" or "LIVE"), 24, 105, 2)
    for i, point in ipairs(state.trail) do
        graphics.setColor(0.35, 0.7, 0.67, i / #state.trail * 0.6)
        graphics.rectangle("fill", point.x, point.y, 12, 12)
    end
    graphics.setColor(0.96, 0.78, 0.42)
    graphics.rectangle("fill", state.x, state.y, 16, 16)
end
function game.keyPressed(key)
    if key == "r" and #replay.stats().snapshots > 0 then replay.rewind(2)
    elseif key == "space" then
        if replay.stats().paused then replay.resume() else replay.pause() end
    elseif key == "n" then replay.step()
    elseif key == "escape" then window.close() end
end
