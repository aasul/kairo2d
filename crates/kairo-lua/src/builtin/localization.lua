local localization = {}
local language, fallback, cache = "en", "en", {}
local function valid(name)
    assert(type(name) == "string" and #name > 0 and #name <= 32 and name:match("^[%w_-]+$"), "invalid locale identifier")
end
local function load(name)
    if not cache[name] then
        local path = "locales/" .. name .. ".json"
        local entries = filesystem.exists(path) and filesystem.readJson(path) or {}
        assert(type(entries) == "table", "locale must be a JSON object")
        local count = 0
        for key, value in pairs(entries) do
            count = count + 1
            assert(count <= 10000 and type(key) == "string" and type(value) == "string" and #value <= 8192, "invalid locale entry")
        end
        cache[name] = entries
    end
    return cache[name]
end
function localization.setLanguage(name) valid(name); load(name); language = name end
function localization.setFallback(name) valid(name); load(name); fallback = name end
function localization.getLanguage() return language end
function localization.reload() cache = {} end
function localization.get(key, values)
    assert(type(key) == "string", "localization key must be a string")
    local text = load(language)[key] or load(fallback)[key] or key
    if values then
        text = text:gsub("{([%w_]+)}", function(name)
            local value = values[name]
            if value == nil then return "{" .. name .. "}" end
            assert(type(value) == "string" or type(value) == "number" or type(value) == "boolean", "interpolation expects scalar values")
            return tostring(value)
        end)
    end
    return text
end
return localization
