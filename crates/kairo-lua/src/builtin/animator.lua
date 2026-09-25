local animator = {}
local Machine = {}
Machine.__index = Machine
local function valid_name(name)
    assert(type(name) == "string" and #name > 0 and #name <= 64, "invalid animation state name")
end
function animator.new()
    return setmetatable({states = {}, edges = {}, active = nil, speed = 1, callbacks = {}}, Machine)
end
function Machine:add(name, clip, options)
    valid_name(name)
    assert(not self.states[name], "animation state already exists")
    assert(type(clip) == "userdata" or type(clip) == "table", "state requires an animation clip")
    assert(type(clip.update) == "function" and type(clip.restart) == "function", "invalid animation clip")
    options = options or {}
    if options.looping ~= nil then clip:setLooping(options.looping) end
    self.states[name] = {clip = clip, enter = options.enter, leave = options.leave}
    if not self.active then self:set(name) end
    return self
end
function Machine:set(name, restart)
    valid_name(name)
    local next_state = assert(self.states[name], "unknown animation state: " .. name)
    if self.active == name and not restart then return end
    local previous = self.active
    if previous and self.states[previous].leave then self.states[previous].leave(previous, name) end
    self.active = name
    next_state.clip:restart()
    next_state.clip:setSpeed(self.speed)
    if next_state.enter then next_state.enter(name, previous) end
    if self.onTransition then self.onTransition(previous, name) end
end
function Machine:transition(from, to, condition)
    assert(self.states[from] and self.states[to], "register both animation states before adding a transition")
    assert(type(condition) == "function" or condition == "finished", "transition expects a predicate or 'finished'")
    assert(#self.edges < 256, "animation transition limit reached")
    self.edges[#self.edges + 1] = {from = from, to = to, test = condition}
    return self
end
function Machine:update(dt)
    if not self.active then return end
    local clip = self.states[self.active].clip
    clip:update(dt)
    for _, edge in ipairs(self.edges) do
        if edge.from == self.active then
            local take = edge.test == "finished" and clip:isFinished()
            if type(edge.test) == "function" then take = edge.test() end
            if take then self:set(edge.to); break end
        end
    end
end
function Machine:setSpeed(speed)
    assert(type(speed) == "number" and speed == speed and speed >= 0 and speed <= 100, "speed must be 0..100")
    self.speed = speed
    for _, state in pairs(self.states) do state.clip:setSpeed(speed) end
end
function Machine:current() return self.active end
function Machine:pause() if self.active then self.states[self.active].clip:pause() end end
function Machine:resume() if self.active then self.states[self.active].clip:resume() end end
function Machine:draw(...)
    if self.active then graphics.draw(self.states[self.active].clip, ...) end
end
return animator
