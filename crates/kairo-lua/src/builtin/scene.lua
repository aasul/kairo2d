local scene = {shared = {}}
local registered, stack, pending = {}, {}, {}
local flushing = false
local callbacks = {"enter", "leave", "suspend", "resume", "update", "draw",
    "keyPressed", "keyReleased", "mousePressed", "mouseReleased", "mouseMoved",
    "gamepadPressed", "gamepadReleased", "gamepadConnected", "gamepadDisconnected"}

local function named(name)
    assert(type(name) == "string" and #name > 0 and #name <= 64, "scene name must contain 1..64 bytes")
    return name
end
local function invoke(item, callback, ...)
    local fn = item and item[callback]
    if fn then fn(item, ...) end
end
local function find(name)
    return assert(registered[named(name)], "scene '" .. name .. "' is not registered")
end
local function enqueue(op, name)
    if name then find(name) end
    assert(#pending < 32, "too many queued scene changes")
    pending[#pending + 1] = {op = op, name = name}
end

function scene.new(name)
    return {name = named(name), opaque = false, pause_physics = false}
end
function scene.register(item)
    assert(type(item) == "table", "scene.register expects a scene table")
    named(item.name)
    assert(not registered[item.name], "scene already registered: " .. item.name)
    for _, name in ipairs(callbacks) do
        assert(item[name] == nil or type(item[name]) == "function", "scene callback must be a function: " .. name)
    end
    local count = 0
    for _ in pairs(registered) do count = count + 1 end
    assert(count < 64, "scene registration limit reached")
    registered[item.name] = item
    return item
end
function scene.switch(name) enqueue("switch", name) end
function scene.push(name) enqueue("push", name) end
function scene.pop() enqueue("pop") end
function scene.reload() enqueue("reload") end
function scene.current() return stack[#stack] and stack[#stack].name end
function scene.info()
    local names, active = {}, {}
    for name in pairs(registered) do names[#names + 1] = name end
    table.sort(names)
    for i, item in ipairs(stack) do active[i] = item.name end
    return {registered = names, stack = active, current = scene.current() or ""}
end
function scene._physicsPaused()
    return stack[#stack] ~= nil and stack[#stack].pause_physics == true
end

-- Changes requested inside callbacks take effect between callbacks, not midway
-- through iteration over a scene stack. Recursive enter/leave chains are bounded.
function scene._flush()
    if flushing then return end
    flushing = true
    local ok, message = pcall(function()
        local processed = 0
        while #pending > 0 do
            processed = processed + 1
            assert(processed <= 32, "recursive scene transition limit reached")
            local change = table.remove(pending, 1)
            local top = stack[#stack]
            if change.op == "switch" then
                while #stack > 0 do invoke(table.remove(stack), "leave") end
                local next_scene = find(change.name)
                stack[1] = next_scene
                invoke(next_scene, "enter")
            elseif change.op == "push" then
                assert(#stack < 16, "scene stack limit reached")
                for _, item in ipairs(stack) do assert(item.name ~= change.name, "scene already on stack") end
                invoke(top, "suspend")
                local next_scene = find(change.name)
                stack[#stack + 1] = next_scene
                invoke(next_scene, "enter")
            elseif change.op == "pop" then
                assert(#stack > 1, "cannot pop the last scene")
                invoke(table.remove(stack), "leave")
                invoke(stack[#stack], "resume")
            else
                assert(top, "no current scene to reload")
                invoke(top, "leave")
                invoke(top, "enter")
            end
        end
    end)
    flushing = false
    if not ok then pending = {}; error(message, 0) end
end
function scene._dispatch(callback, ...)
    if callback ~= "draw" then scene._flush() end
    if callback == "draw" then
        local first = 1
        for i = #stack, 1, -1 do if stack[i].opaque then first = i; break end end
        for i = first, #stack do invoke(stack[i], "draw", ...) end
    elseif callback ~= "load" and callback ~= "quit" and callback ~= "debugUI" then
        invoke(stack[#stack], callback, ...)
    end
    if callback ~= "draw" then scene._flush() end
end
return scene
