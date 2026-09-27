local Enemy = {}

function Enemy.ready(self)
    self.phase = 0
end

function Enemy.update(self, dt)
    self.phase = self.phase + dt
    local result = 0
    for i = 1, 100 do
        result = result + math.sin(self.phase + i * 0.01)
    end
    self.last_result = result
end

return Enemy
