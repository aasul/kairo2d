local host = ...
local roots = {}
local replay = {}

local function encode(value, seen, depth, count)
    count.n = count.n + 1
    assert(count.n <= 20000 and depth <= 24, "replay state exceeds node/depth limits")
    local kind = type(value)
    if kind == "number" then
        assert(value == value and value ~= math.huge and value ~= -math.huge, "replay numbers must be finite")
        return {kind = "number", value = value}
    elseif kind == "string" then
        count.bytes = count.bytes + #value
        assert(count.bytes <= 4 * 1024 * 1024, "replay strings exceed 4 MiB")
        return {kind = "string", value = value}
    elseif kind == "boolean" then
        return {kind = "boolean", value = value}
    elseif kind == "table" then
        assert(getmetatable(value) == nil, "replay only accepts plain tables without metatables")
        assert(not seen[value], "replay does not support cycles or shared table aliases")
        seen[value] = true
        local entries = {}
        for key, child in pairs(value) do
            assert(type(key) == "string" or type(key) == "number" or type(key) == "boolean", "unsupported replay table key")
            entries[#entries + 1] = {key = encode(key, seen, depth + 1, count), value = encode(child, seen, depth + 1, count)}
        end
        return {kind = "table", entries = entries}
    end
    error("cannot capture replay value of type " .. kind, 3)
end

local function decode(node)
    if node.kind ~= "table" then return node.value end
    local result = {}
    for _, entry in ipairs(node.entries or {}) do result[decode(entry.key)] = decode(entry.value) end
    return result
end

local function merge(target, source)
    for key in pairs(target) do if source[key] == nil then target[key] = nil end end
    for key, value in pairs(source) do
        if type(value) == "table" and type(target[key]) == "table" then
            merge(target[key], value)
        else
            target[key] = value
        end
    end
end

function replay.register(name, state)
    assert(type(name) == "string" and #name > 0 and #name <= 64, "replay name must contain 1..64 bytes")
    assert(type(state) == "table", "replay.register expects a table")
    roots[name] = state
    host.clear()
end

function replay.capture()
    host.store(encode(roots, {}, 0, {n = 0, bytes = 0}))
end
function replay.bookmark(label, note)
    return host.bookmark(label, note or "", encode(roots, {}, 0, {n=0,bytes=0}))
end
function replay.getBookmarks() return host.bookmarks() end
function replay.enable(enabled) host.enable(enabled) end
function replay.clear() host.clear() end
function replay.stats() return host.stats() end
function replay.rewind(seconds) host.request("rewind", seconds) end
function replay.seek(index) host.request("seek", index) end
function replay.pause() host.request("pause", 0) end
function replay.resume() host.request("resume", 0) end
function replay.step() host.request("step", 0) end

-- Called at frame boundaries by the runtime, never in the middle of a callback.
function replay._before()
    local snapshot = host.restore()
    if snapshot then
        merge(roots, decode(snapshot))
        if game and game.replayRestored then game.replayRestored() end
    end
end
function replay._after(dt)
    if host.due(dt) then replay.capture() end
end
function replay._encode(value) return encode(value, {}, 0, {n = 0, bytes = 0}) end
function replay._decode(value) return decode(value) end
return replay
