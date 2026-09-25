local host = ...
local inspector, fields = {}, {}
local function plain(value, seen, depth, count)
    count.n = count.n + 1
    assert(count.n <= 64 and depth <= 4, "inspector value is too large/deep")
    local kind = type(value)
    assert(kind == "number" or kind == "string" or kind == "boolean" or kind == "table", "unsupported inspector value: " .. kind)
    if kind == "number" then assert(value == value and math.abs(value) ~= math.huge, "non-finite inspector number") end
    if kind == "string" then assert(#value <= 512, "inspector string is too long") end
    if kind == "table" then
        assert(getmetatable(value) == nil and not seen[value], "inspector tables cannot have metatables, cycles or aliases")
        seen[value] = true
        for key, child in pairs(value) do
            assert(type(key) == "string" or (type(key) == "number" and key % 1 == 0 and key > 0 and key <= 64), "invalid inspector table key")
            plain(child, seen, depth + 1, count)
        end
    end
end
local function describe(path, field)
    local value = rawget(field.target, field.key)
    plain(value, {}, 0, {n = 0})
    return host.describe(path, value, field.metadata)
end
function inspector.expose(path, target, key, metadata)
    assert(type(target) == "table" and getmetatable(target) == nil, "inspector target must be a plain table")
    assert(type(key) == "string" or type(key) == "number", "inspector key must be a string or number")
    assert(not fields[path], "inspector path is already exposed")
    local count = 0
    for _ in pairs(fields) do count = count + 1 end
    assert(count < 64, "inspector field limit reached")
    metadata = metadata or {}
    local copy = {}
    for name, value in pairs(metadata) do copy[name] = value end
    copy.kind = copy.kind or type(rawget(target, key))
    local field = {target = target, key = key, metadata = copy}
    describe(path, field)
    fields[path] = field
end
function inspector.remove(path) fields[path] = nil end
function inspector.clear() fields = {} end
function inspector._snapshot()
    local nodes = {}
    for path, field in pairs(fields) do nodes[#nodes + 1] = describe(path, field) end
    table.sort(nodes, function(a,b) return a.path < b.path end)
    return nodes
end
local function merge(target, source)
    for key, value in pairs(source) do
        if type(value) == "table" then merge(target[key], value) else rawset(target, key, value) end
    end
end
function inspector._apply(update)
    local field = assert(fields[update.path], "inspector path is not exposed")
    local node = describe(update.path, field)
    host.validateUpdate(node, update)
    if type(update.value) == "table" then merge(rawget(field.target, field.key), update.value)
    else rawset(field.target, field.key, update.value) end
end
return inspector
