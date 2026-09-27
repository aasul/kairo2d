local Player = {}

function Player.ready(self)
    self.steps = 0
    self.visitToken = Events.on('scene_entered', function(visits)
        self.steps = visits
    end, self.node)
end

function Player.update(self)
    self.node:setProperty('velocity', {
        x = input.axis('move_x') * 180,
        y = input.axis('move_y') * 180,
    })
end

return Player
