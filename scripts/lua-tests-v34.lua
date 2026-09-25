-- These tests execute the actual bundled Lua libraries. Engine services below
-- are explicit test doubles; native bindings have separate Rust tests.
local root = SOURCE_ROOT .. "/crates/kairo-lua/src/builtin/"
local function library(name, ...) return assert(loadfile(root .. name .. ".lua"))(... ) end
local function fails(fn, pattern)
    local ok, err = pcall(fn)
    assert(not ok and tostring(err):find(pattern, 1, true), tostring(err))
end

do
    local scene = library("scene")
    local trace = {}
    local function note(s) trace[#trace + 1] = s end
    local game, pause = scene.new("game"), scene.new("pause")
    game.enter = function() note("enter") end
    game.leave = function() note("leave") end
    game.update = function() note("update") end
    game.draw = function() note("game.draw") end
    game.suspend = function() note("suspend") end
    game.resume = function() note("resume") end
    pause.pause_physics = true
    pause.enter = function() note("pause.enter") end
    pause.update = function() note("pause.update") end
    pause.draw = function() note("pause.draw") end
    pause.leave = function() note("pause.leave") end
    scene.register(game); scene.register(pause)
    scene.switch("game"); assert(scene.current() == nil)
    scene._dispatch("load")
    scene._dispatch("update", 0.1)
    scene.push("pause"); scene._flush()
    assert(scene.current() == "pause" and scene._physicsPaused())
    scene._dispatch("update", 0.1); scene._dispatch("draw")
    assert(table.concat(trace, ",") == "enter,update,suspend,pause.enter,pause.update,game.draw,pause.draw")
    pause.opaque = true; trace = {}; scene._dispatch("draw")
    assert(table.concat(trace) == "pause.draw")
    scene.pop(); scene._flush(); assert(scene.current() == "game" and not scene._physicsPaused())
    scene.reload(); scene._flush(); assert(trace[#trace] == "enter")
    fails(function() scene.switch("missing") end, "not registered")
    scene.pop(); fails(scene._flush, "cannot pop")
    game.enter = function() scene.reload() end
    scene.reload(); fails(scene._flush, "recursive scene")
    game.enter = nil; scene.reload(); scene._flush()
    assert(scene.current() == "game")
    print("Lua 3.4: scene lifecycle, overlays, pause, queue boundaries and recovery passed")
end

do
    local animator = library("animator")
    local function clip()
        return {t=0, speed=1, paused=false,
            update=function(self,dt) if not self.paused then self.t=self.t+dt*self.speed end end,
            restart=function(self) self.t=0; self.paused=false end,
            setSpeed=function(self,v) self.speed=v end,
            setLooping=function(self,v) self.loop=v end,
            isFinished=function(self) return self.t >= 0.2 end,
            pause=function(self) self.paused=true end, resume=function(self) self.paused=false end}
    end
    local idle, attack = clip(), clip()
    local machine = animator.new():add("idle", idle):add("attack", attack, {looping=false})
    local trigger = false
    machine:transition("idle", "attack", function() return trigger end)
    machine:transition("attack", "idle", "finished")
    machine:update(0.1); assert(machine:current() == "idle")
    trigger=true; machine:update(0.1); trigger=false; assert(machine:current() == "attack" and attack.t==0)
    machine:setSpeed(2); machine:update(0.1); assert(machine:current() == "idle")
    machine:pause(); machine:update(0.1); assert(idle.t==0)
    machine:resume(); machine:update(0.1); assert(idle.t==0.2)
    fails(function() machine:set("unknown") end, "unknown animation")
    fails(function() machine:setSpeed(0/0) end, "speed must")
    print("Lua 3.4: animation predicates, completion transitions, pause and speed passed")
end

do
    local json = { ["locales/en.json"]={play="Play {name}",only="Fallback"}, ["locales/fr.json"]={play="Jouer {name}"} }
    filesystem = { exists=function(path) return json[path] ~= nil end, readJson=function(path) return json[path] end }
    local loc = library("localization")
    loc.setLanguage("fr")
    assert(loc.get("play", {name="Ada"}) == "Jouer Ada")
    assert(loc.get("only") == "Fallback" and loc.get("missing") == "missing")
    fails(function() loc.setLanguage("../../oops") end, "locale identifier")
    print("Lua 3.4: localization fallback, interpolation and path validation passed")
end

do
    local source = {defaults={health=10,stats={speed=2},texture="assets/player.png"}}
    filesystem = {readToml=function(path) assert(path=="prefabs/enemy.toml"); return source end}
    local prefab = library("prefab")
    local a = prefab.spawn("enemy", {stats={speed=5},x=30})
    local b = prefab.spawn("enemy")
    assert(a.health==10 and a.stats.speed==5 and b.stats.speed==2 and source.defaults.stats.speed==2)
    a.stats.speed=100; assert(b.stats.speed==2)
    fails(function() prefab.spawn("../escape") end, "prefab")
    print("Lua 3.4: prefab deep overrides, isolation and path validation passed")
end

do
    local data, writes = nil, 0
    local save = library("save_migrations", {read=function() return data end, write=function(_,value) data=value; writes=writes+1 end})
    save.registerMigration(1,2,function(value) return {coins=value.coins,health=100} end)
    save.registerMigration(2,3,function(value) value.level=1; return value end)
    save.writeVersioned("player.json",1,{coins=9})
    local upgraded=save.readVersioned("player.json",3)
    assert(upgraded.health==100 and upgraded.level==1 and upgraded.coins==9 and writes==1)
    fails(function() save.readVersioned("player.json",4) end, "missing save migration")
    save.writeVersioned("player.json",5,{})
    fails(function() save.readVersioned("player.json",3) end, "newer game")
    fails(function() save.registerMigration(1,2,function(v) return v end) end, "already registered")
    print("Lua 3.4: ordered save migrations, read-only migration and incompatible version rejection passed")
end

do
    local host = {
        describe=function(path,value,metadata) return {path=path,value=value,metadata=metadata} end,
        validateUpdate=function(node,update)
            assert(node.metadata.writable==true, "read only")
            assert(type(node.value)==type(update.value), "type mismatch")
        end,
    }
    local inspector=library("inspector",host)
    local player={speed=20,position={x=1,y=2}}
    local alias=player.position
    inspector.expose("player.speed",player,"speed",{writable=true,min=0,max=100})
    inspector.expose("player.position",player,"position",{writable=true})
    inspector._apply({path="player.speed",value=30}); assert(player.speed==30)
    inspector._apply({path="player.position",value={x=4,y=5}})
    assert(player.position==alias and alias.x==4)
    local bad={};bad.self=bad;player.bad=bad
    fails(function() inspector.expose("bad",player,"bad") end,"cycles or aliases")
    player.fn=function() end
    fails(function() inspector.expose("fn",player,"fn") end,"unsupported inspector")
    inspector.remove("player.speed"); assert(#inspector._snapshot()==1)
    print("Lua 3.4: explicit inspector registration, in-place edits and unsupported value rejection passed (host validation doubled)")
end
