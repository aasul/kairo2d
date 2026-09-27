local scene = {shared = {}}
local registered, stack, pending = {}, {}, {}
local flushing = false
local callbacks = {"enter", "leave", "suspend", "resume", "update", "draw",
    "keyPressed", "keyReleased", "mousePressed", "mouseReleased", "mouseMoved",
    "gamepadPressed", "gamepadReleased", "gamepadConnected", "gamepadDisconnected"}
local scriptCallbacks = {"ready", "update", "fixedUpdate", "draw", "onDestroy",
    "onCollision", "onTriggerEnter", "onTriggerExit", "onClick"}
local pressedControl
local profile_invoke = profiler._invoke

local function named(name)
    assert(type(name) == "string" and #name > 0 and #name <= 64, "scene name must contain 1..64 bytes")
    return name
end
local function invoke(item, callback, ...)
    local fn = item and item[callback]
    if fn then
        if profile_invoke then
            profile_invoke("@scene", item.name, item.name, callback, fn, item, ...)
        else
            fn(item, ...)
        end
    end
end
local function find(name)
    return assert(registered[named(name)], "scene '" .. name .. "' is not registered")
end
local function enqueue(op, name)
    if name then find(name) end
    assert(#pending < 32, "too many queued scene changes")
    pending[#pending + 1] = {op = op, name = name}
end

function scene.createNode(item, nodeType, nodeName)
    return item.root:createChild(nodeType, nodeName)
end
function scene.new(name)
    local graph = sceneGraph.new(named(name))
    return {
        name = name, graph = graph, root = graph:root(),
        opaque = false, pause_physics = false,
        createNode = scene.createNode,
        attachScript = scene.attachScript,
    }
end
function scene.load(path)
    local graph = sceneGraph.load(path)
    local item = scene.new(graph:name())
    item.graph, item.root = graph, graph:root()
    item.source = path
    return item
end
local function invoke_script(item, record, callback, ...)
    local fn = record.callbacks[callback]
    if not fn then return end
    local ok, message
    if profile_invoke then
        ok, message = pcall(profile_invoke, record.script, item.name, record.path,
            callback, fn, record.state, ...)
    else
        ok, message = pcall(fn, record.state, ...)
    end
    if not ok then
        error(("scene '%s', node '%s', script '%s', callback '%s': %s")
            :format(item.name, record.path, record.script, callback, tostring(message)), 0)
    end
end
function scene.attachScript(item, node, path)
    assert(type(item) == "table" and item.graph and item.root, "attachScript expects a scene")
    assert(type(path) == "string", "script path must be a string")
    node.script = path -- Rust validates the project-relative path.
    item._script_nodes = item._script_nodes or {}
    if item._script_nodes[node.id] then
        assert(item._script_nodes[node.id].script == path, "node already has a different script instance")
        return item._script_nodes[node.id].state
    end
    local module = path:gsub("%.lua$", ""):gsub("/", ".")
    local definition = require(module)
    assert(type(definition) == "table", "node script must return a table: " .. path)
    local record = {node = node, script = path, path = node:path(), state = {node = node}, callbacks = {}}
    for _, callback in ipairs(scriptCallbacks) do
        local fn = definition[callback]
        assert(fn == nil or type(fn) == "function", "script callback must be a function: " .. callback)
        if fn then record.callbacks[callback] = fn end
    end
    item._script_nodes[node.id] = record
    item._script_callbacks = item._script_callbacks or {}
    if record.callbacks.onDestroy then
        item._destroy_watch = item._destroy_watch or {}
        item._destroy_watch[#item._destroy_watch + 1] = record
    end
    for callback in pairs(record.callbacks) do
        local list = item._script_callbacks[callback] or {}
        list[#list + 1] = record
        item._script_callbacks[callback] = list
    end
    return record.state
end
local function install_scripts(item)
    if not item.root then return end
    local pending = {item.root}
    while #pending > 0 do
        local node = table.remove(pending)
        if node.script then scene.attachScript(item, node, node.script) end
        for _, child in ipairs(node:children()) do pending[#pending + 1] = child end
    end
end
local function dispatch_scripts(item, callback, ...)
    if not item or not item._script_callbacks then return end
    local stale = false
    if item._destroy_watch then
        for _, record in ipairs(item._destroy_watch) do
            if not record.destroyed and not record.node:isAlive() then
                record.destroyed = true
                stale = true
                invoke_script(item, record, "onDestroy")
            end
        end
    end
    local list = item._script_callbacks[callback] or {}
    for _, record in ipairs(list) do
        if not record.node:isAlive() then
            stale = true
            if not record.destroyed then
                record.destroyed = true
                invoke_script(item, record, "onDestroy")
            end
        elseif record.node.enabled then
            if callback == "ready" then
                if not record.ready then
                    record.ready = true
                    invoke_script(item, record, "ready")
                end
            else
                if not record.ready then
                    record.ready = true
                    invoke_script(item, record, "ready")
                end
                invoke_script(item, record, callback, ...)
            end
        end
    end
    if stale then
        for event, records in pairs(item._script_callbacks) do
            local kept = {}
            for _, record in ipairs(records) do
                if not record.destroyed then kept[#kept + 1] = record end
            end
            item._script_callbacks[event] = kept
        end
        for id, record in pairs(item._script_nodes) do
            if record.destroyed then item._script_nodes[id] = nil end
        end
        if item._destroy_watch then
            local kept = {}
            for _, record in ipairs(item._destroy_watch) do
                if not record.destroyed then kept[#kept + 1] = record end
            end
            item._destroy_watch = kept
        end
    end
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
    install_scripts(item)
    registered[item.name] = item
    return item
end
local function scene_name(value)
    if type(value) == "table" then
        if not registered[value.name] then scene.register(value) end
        assert(registered[value.name] == value, "a different scene is registered with this name")
        return value.name
    end
    return value
end
function scene.switch(name) enqueue("switch", scene_name(name)) end
function scene.push(name) enqueue("push", scene_name(name)) end
function scene.pop() enqueue("pop") end
function scene.reload() enqueue("reload") end
function scene.current() return stack[#stack] and stack[#stack].name end
function scene.info()
    local names, active = {}, {}
    local active_nodes = 0
    for name in pairs(registered) do names[#names + 1] = name end
    table.sort(names)
    for i, item in ipairs(stack) do
        active[i] = item.name
        if item.graph then active_nodes = active_nodes + item.graph:_nodeCount() end
    end
    return {registered = names, stack = active, current = scene.current() or "", active_nodes = active_nodes}
end
function scene._runtimeSnapshot(offset, graph_id, file_id)
    local top = stack[#stack]
    if not top or not top.graph then return nil end
    local snapshot = top.graph:_runtimePage(offset, graph_id, file_id)
    snapshot.source = top.source
    return snapshot
end
function scene._runtimeContains(graph_id, file_id)
    local top = stack[#stack]
    return top ~= nil and top.graph ~= nil and top.graph:_runtimeContains(graph_id, file_id)
end

function scene._runtimeEdit(graph_id, file_id, update)
    local top = stack[#stack]
    assert(top and top.graph, "no active scene graph")
    return top.graph:_runtimeEdit(graph_id, file_id, update)
end
function scene._physicsPaused()
    return stack[#stack] ~= nil and stack[#stack].pause_physics == true
end
function scene._nativeUpdate()
    local top = stack[#stack]
    if top and top.graph then top.graph:syncAudio() end
end
function scene._nativeBeforePhysics()
    local top = stack[#stack]
    if top and top.graph then top.graph:syncPhysicsBefore() end
end
function scene._nativeAfterPhysics()
    local top = stack[#stack]
    if not top or not top.graph then return end
    top.graph:syncPhysicsAfter()
    for _, pair in ipairs(top.graph:collisionPairs()) do
        local a, b = pair[1], pair[2]
        if a:isAlive() and b:isAlive() then
            local record = top._script_nodes and top._script_nodes[a.id]
            a:emit('collision', b)
            if record and a:isAlive() then invoke_script(top, record, 'onCollision', b) end
        end
        if a:isAlive() and b:isAlive() then
            local record = top._script_nodes and top._script_nodes[b.id]
            b:emit('collision', a)
            if record and b:isAlive() then invoke_script(top, record, 'onCollision', a) end
        end
    end
end
function scene._pointer(down, x, y, button)
    if button ~= 1 then return end
    local item = stack[#stack]
    local target = item and item.graph and item.graph:hitControl(x, y)
    if down then
        pressedControl = target and {scene = item, key = target:_signalKey()}
    else
        local pressed = pressedControl
        pressedControl = nil
        if pressed and pressed.scene == item and target and target:_signalKey() == pressed.key then
            local record = item._script_nodes and item._script_nodes[target.id]
            target:emit('click', x, y)
            if record and target:isAlive() then invoke_script(item, record, 'onClick', x, y) end
        end
    end
end
function scene._cancelPointer() pressedControl = nil end
local function deactivate_native(item)
    if item and item.graph then
        item.graph:deactivatePhysics()
        item.graph:deactivateAudio()
    end
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
                while #stack > 0 do
                    local old = table.remove(stack)
                    invoke(old, "leave")
                    deactivate_native(old)
                end
                local next_scene = find(change.name)
                stack[1] = next_scene
                invoke(next_scene, "enter")
                dispatch_scripts(next_scene, "ready")
            elseif change.op == "push" then
                assert(#stack < 16, "scene stack limit reached")
                for _, item in ipairs(stack) do assert(item.name ~= change.name, "scene already on stack") end
                invoke(top, "suspend")
                deactivate_native(top)
                local next_scene = find(change.name)
                stack[#stack + 1] = next_scene
                invoke(next_scene, "enter")
                dispatch_scripts(next_scene, "ready")
            elseif change.op == "pop" then
                assert(#stack > 1, "cannot pop the last scene")
                local old = table.remove(stack)
                invoke(old, "leave")
                deactivate_native(old)
                invoke(stack[#stack], "resume")
            else
                assert(top, "no current scene to reload")
                invoke(top, "leave")
                deactivate_native(top)
                invoke(top, "enter")
                dispatch_scripts(top, "ready")
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
        for i = first, #stack do
            invoke(stack[i], "draw", ...)
            if stack[i].graph then stack[i].graph:drawNative() end
            dispatch_scripts(stack[i], "draw", ...)
        end
    elseif callback ~= "load" and callback ~= "quit" and callback ~= "debugUI" then
        invoke(stack[#stack], callback, ...)
        dispatch_scripts(stack[#stack], callback, ...)
    end
    if callback ~= "draw" then scene._flush() end
end
return scene
