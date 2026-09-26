local Player = {}
Player.__index = Player

function Player.new()
    return setmetatable({x = 100, y = 180, speed = 240}, Player)
end

function Player:update(dt)
    local dx, dy = 0, 0
    if keyboard.isDown("a") or keyboard.isDown("left") then dx = dx - 1 end
    if keyboard.isDown("d") or keyboard.isDown("right") then dx = dx + 1 end
    if keyboard.isDown("w") or keyboard.isDown("up") then dy = dy - 1 end
    if keyboard.isDown("s") or keyboard.isDown("down") then dy = dy + 1 end
    local length = math.sqrt(dx * dx + dy * dy)
    if length > 0 then
        self.x = self.x + dx / length * self.speed * dt
        self.y = self.y + dy / length * self.speed * dt
    end
    local width, height = window.getSize()
    self.x = math.max(0, math.min(width - 64, self.x))
    self.y = math.max(0, math.min(height - 64, self.y))
end

return Player
