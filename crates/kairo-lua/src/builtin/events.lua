-- One Lua-owned signal registry per game VM. Native nodes provide validated keys.
local Events = {}
local listeners, byKey = {}, {}
local nextId, count, depth = 0, 0, 0

local function eventName(name)
    assert(type(name) == "string" and #name > 0 and #name <= 64,
        "event name must contain 1..64 bytes")
    return name
end

local function key(source, name)
    return source .. "\0" .. eventName(name)
end

local function add(source, name, callback, owner)
    assert(type(callback) == "function", "event callback must be a function")
    assert(count < 4096, "event subscription limit reached")
    assert(nextId < 9007199254740991, "event subscription IDs exhausted")
    nextId = nextId + 1
    local signal = key(source, name)
    local record = {id = nextId, source = source, signal = signal,
        owner = owner, callback = callback}
    listeners[nextId] = record
    byKey[signal] = byKey[signal] or {}
    byKey[signal][#byKey[signal] + 1] = nextId
    count = count + 1
    return nextId
end

function Events.off(id)
    local record = listeners[id]
    if not record then return false end
    listeners[id] = nil
    count = count - 1
    local group = byKey[record.signal]
    for i, token in ipairs(group) do
        if token == id then table.remove(group, i); break end
    end
    if #group == 0 then byKey[record.signal] = nil end
    return true
end

local function emit(source, name, ...)
    local signal = key(source, name)
    local group = byKey[signal]
    if not group then return end
    assert(depth < 32, "recursive event dispatch limit reached")
    local snapshot = {}
    for i, id in ipairs(group) do snapshot[i] = id end
    local args = table.pack(...)
    depth = depth + 1
    local ok, message = pcall(function()
        for _, id in ipairs(snapshot) do
            local record = listeners[id]
            if record then record.callback(table.unpack(args, 1, args.n)) end
        end
    end)
    depth = depth - 1
    if not ok then error("event '" .. name .. "': " .. tostring(message), 0) end
end

function Events.on(name, callback, owner)
    local ownerKey = owner and owner:_signalKey() or nil
    return add("global", name, callback, ownerKey)
end

function Events.emit(name, ...)
    return emit("global", name, ...)
end

function Events._onNode(nodeKey, name, callback)
    return add("node:" .. nodeKey, name, callback, nodeKey)
end

function Events._emitNode(nodeKey, name, ...)
    return emit("node:" .. nodeKey, name, ...)
end

function Events._destroy(nodeKeys)
    local removed = {}
    for _, nodeKey in ipairs(nodeKeys) do removed[nodeKey] = true end
    local pending = {}
    for id, record in pairs(listeners) do
        if removed[record.owner] or removed[record.source:match("^node:(.*)$")] then
            pending[#pending + 1] = id
        end
    end
    for _, id in ipairs(pending) do Events.off(id) end
end

return Events
