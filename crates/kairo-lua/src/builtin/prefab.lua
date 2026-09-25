local prefab = {}
local function clone(value, depth, count)
    count.n = count.n + 1
    assert(depth <= 12 and count.n <= 4096, "prefab data exceeds limits")
    if type(value) ~= "table" then return value end
    local out = {}
    for key, item in pairs(value) do out[key] = clone(item, depth + 1, count) end
    return out
end
local function merge(target, overrides, depth)
    assert(depth <= 12, "prefab overrides exceed depth limit")
    for key, value in pairs(overrides) do
        if type(value) == "table" and type(target[key]) == "table" then merge(target[key], value, depth + 1)
        else target[key] = clone(value, 0, {n = 0}) end
    end
end
function prefab.spawn(name, overrides)
    assert(type(name) == "string" and #name <= 128 and name:match("^[%w_/-]+$") and not name:find("..", 1, true), "invalid prefab name")
    local definition = filesystem.readToml("prefabs/" .. name .. ".toml")
    assert(type(definition.defaults) == "table", "prefab requires a [defaults] table")
    local data = clone(definition.defaults, 0, {n = 0})
    if overrides then merge(data, clone(overrides, 0, {n = 0}), 0) end
    if definition.factory then
        local factory = require(definition.factory)
        assert(type(factory) == "function", "prefab factory module must return a function")
        return factory(data)
    end
    return data
end
return prefab
