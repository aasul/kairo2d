local save = ...
local migrations = {}
local function version(number)
    assert(type(number) == "number" and number % 1 == 0 and number >= 1 and number <= 10000, "save version must be an integer in 1..10000")
end
function save.registerMigration(from, to, callback)
    version(from); version(to)
    assert(to == from + 1 and type(callback) == "function", "register adjacent save versions with a migration function")
    assert(not migrations[from], "migration already registered")
    migrations[from] = callback
end
function save.writeVersioned(name, format, data)
    version(format)
    assert(type(data) == "table", "versioned save data must be a table")
    save.write(name, {kairo_save_version = format, data = data})
end
function save.readVersioned(name, target)
    version(target)
    local envelope = save.read(name)
    if envelope == nil then return nil end
    assert(type(envelope) == "table", "not a versioned save")
    version(envelope.kairo_save_version)
    assert(type(envelope.data) == "table", "invalid versioned save data")
    assert(envelope.kairo_save_version <= target, "save was written by a newer game version")
    local data = envelope.data
    for current = envelope.kairo_save_version, target - 1 do
        local migration = assert(migrations[current], "missing save migration from version " .. current)
        data = migration(data)
        assert(type(data) == "table", "save migration must return a table")
    end
    -- Reads never overwrite disk. Persist only after the game accepts the result.
    return data
end
return save
