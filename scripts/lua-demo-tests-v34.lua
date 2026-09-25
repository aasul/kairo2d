-- Executes shipped Lua source. Graphics, audio, physics, assets and Rust APIs
-- below are explicit test doubles, not a substitute for native integration tests.
local fixtures=assert(load(DEMO_FIXTURES,'fixtures','t',{}))()
local function clone(value)
    if type(value)~='table' then return value end
    local result={};for k,v in pairs(value)do result[k]=clone(v)end;return result
end
local function boot(name)
    local prefix='examples/'..name..'/'
    local env=setmetatable({game={},draws={},sounds=0,bodies={},down={},pressed={},mark_count=0},{__index=_G})
    env._G=env
    local phase='load'
    local function asset(path)return clone(assert(fixtures[prefix..path],'missing fixture: '..prefix..path))end
    local function module(name,...)
        return assert(loadfile(SOURCE_ROOT..'/crates/kairo-lua/src/builtin/'..name..'.lua','t',env))(...)
    end
    env.graphics={}
    for _,name in ipairs({'clear','setColor','rectangle','print','draw','setCamera','resetCamera'})do
        env.graphics[name]=function(...)
            assert(phase=='draw','draw API outside draw callback: '..name)
            for _,v in ipairs({...})do if type(v)=='number'then assert(v==v and math.abs(v)<math.huge)end end
            env.draws[#env.draws+1]={name,...}
        end
    end
    env.graphics.loadTexture=function(path)return {path=path,getDimensions=function()return 64,16 end}end
    env.filesystem={exists=function(path)return fixtures[prefix..path]~=nil end,readJson=asset,readToml=asset}
    env.keyboard={isDown=function(key)return env.down[key] or false end}
    env.mouse={position=function()return 240,240 end}
    env.window={close=function()env.closed=true end}
    local bindings=fixtures[prefix..'input.toml'] or {actions={},axes={}}
    bindings=clone(bindings);bindings.actions=bindings.actions or {};bindings.axes=bindings.axes or {}
    env.input={
        bind=function(name,binding)bindings.actions[name]=binding end,
        bindAxis=function(name,binding)bindings.axes[name]=binding end,
        pressed=function(name)assert(bindings.actions[name],'unknown action '..name);return env.pressed[name] or false end,
        action=function(name)assert(bindings.actions[name]);return env.down[name] or false end,
        axis=function(name)assert(bindings.axes[name],'unknown axis '..name);return env.down[name] or 0 end,
    }
    env.audio={isAvailable=function()return true end,
        loadSound=function(path)assert(fixtures[prefix..path]);return path end,
        play=function()assert(phase~='load','sound played during candidate load');env.sounds=env.sounds+1 end,
        setBusVolume=function()end,muteBus=function()end,stopBus=function()end,
        mixer=function()return{{name='master',volume=1,muted=false},{name='music',volume=0.5,muted=false}}end}
    env.animation={new=function(_,frames)
        local clip={frames=frames,elapsed=0,finished=false,playing=true,speed=1}
        function clip:update(dt)if self.playing then self.elapsed=self.elapsed+dt*self.speed end end
        function clip:restart()self.elapsed=0;self.finished=false;self.playing=true end
        function clip:setSpeed(v)self.speed=v end
        function clip:setLooping(v)self.loop=v end
        function clip:isFinished()return self.finished end
        function clip:pause()self.playing=false end
        function clip:resume()self.playing=true end
        return clip
    end}
    env.particles={new=function(config)
        local emitter={config=config,n=0,x=0,y=0}
        function emitter:setPosition(x,y)self.x=x;self.y=y end
        function emitter:emit(n)self.n=math.min(self.n+n,self.config.max_particles or 1024);return self.n end
        function emitter:clear()self.n=0 end
        function emitter:update(dt)assert(dt>=0)end
        function emitter:count()return self.n end
        function emitter:draw()assert(phase=='draw')end
        return emitter
    end}
    env.particles.load=function(path)return env.particles.new(asset(path).emitter)end
    env.physics={setGravity=function()end,newRectangle=function(kind,x,y,w,h)
        assert(w>0 and h>0)
        local body={kind=kind,x=x,y=y,vx=0,vy=0}
        function body:setPosition(x,y)self.x=x;self.y=y end
        function body:getPosition()return self.x,self.y end
        function body:setVelocity(x,y)self.vx=x;self.vy=y end
        function body:setRotation(r)self.rotation=r end
        env.bodies[#env.bodies+1]=body;return body
    end}
    env.tilemap={load=function(path)
        local map=asset(path)
        function map:getObjects()return self.layers[2].objects end
        function map:getCollisionRects()
            local rects={}
            for _,obj in ipairs(self:getObjects())do
                for _,p in ipairs(obj.properties or{})do if p.name=='collision' and p.value then
                    rects[#rects+1]={x=obj.x+obj.width/2,y=obj.y+obj.height/2,width=obj.width,height=obj.height,rotation=0}
                end end
            end
            return rects
        end
        function map:update(dt)assert(dt>=0)end
        function map:draw()assert(phase=='draw')end
        return map
    end}
    local storage={}
    env.save=module('save_migrations',{read=function(path)return clone(storage[path])end,write=function(path,data)storage[path]=clone(data)end})
    env.scene=module('scene');env.animator=module('animator');env.prefab=module('prefab');env.localization=module('localization')
    env.inspector=module('inspector',{
        describe=function(path,value,metadata)return{path=path,value=value,metadata=metadata}end,
        validateUpdate=function()error('not used in this demo logic test')end,
    })
    env.replay=module('replay',{
        clear=function()end,enable=function()end,request=function()end,stats=function()return{}end,
        store=function()end,due=function()return false end,restore=function()return nil end,
        bookmark=function(label,note,data)assert(type(data)=='table');env.mark_count=env.mark_count+1;return env.mark_count end,
        bookmarks=function()return{}end,
    })
    assert(loadfile(SOURCE_ROOT..'/'..prefix..'main.lua','t',env))()
    if env.game.load then env.game.load()end;env.scene._dispatch('load');phase='ready'
    function env.tick(dt)
        phase='update';if env.game.update then env.game.update(dt)end;env.scene._dispatch('update',dt)
        if not env.scene._physicsPaused()then for _,b in ipairs(env.bodies)do
            if b.kind=='dynamic'then b.x=b.x+b.vx*dt;b.y=b.y+b.vy*dt end
        end end
        env.draws={};phase='draw';if env.game.draw then env.game.draw()end;env.scene._dispatch('draw');phase='ready'
        env.pressed={}
    end
    function env.press_key(key)
        phase='input';if env.game.keyPressed then env.game.keyPressed(key)end;env.scene._dispatch('keyPressed',key);phase='ready'
    end
    return env,storage
end
local game=boot('scenes')
assert(game.scene.current()=='menu');game.pressed.accept=true;game.tick(1/60);assert(game.scene.current()=='play')
game.pressed.pause=true;game.tick(1/60);assert(game.scene.current()=='pause')
local x=game.scene.shared.x;game.tick(1/60);assert(game.scene.shared.x==x)
game.pressed.pause=true;game.tick(1/60);assert(game.scene.current()=='play')
for _,name in ipairs({'input-actions','live-inspector','particles','replay-bugs','platformer'})do
    local demo=boot(name)
    for frame=1,120 do
        demo.down.move=frame<60 and 1 or 0;demo.down.move_x=1;demo.down.move_y=0
        demo.pressed.activate=frame==30;demo.pressed.jump=frame==70;demo.pressed.mark=frame==50
        demo.tick(1/60)
    end
    assert(#demo.draws>0)
    if name=='replay-bugs'then assert(demo.mark_count==1)end
end
local mixer=boot('audio-mixer');for _,key in ipairs({'1','2','3','space'})do mixer.press_key(key)end
assert(mixer.sounds==3);mixer.tick(1/60)
local yard,storage=boot('top-down')
assert(yard.scene.current()=='menu');yard.pressed.accept=true;yard.tick(1/60);assert(yard.scene.current()=='play')
for frame=1,600 do yard.down.move_x=0;yard.down.move_y=0;yard.pressed.mark=frame==100;yard.tick(1/60)end
assert(yard.mark_count==1)
yard.pressed.pause=true;yard.tick(1/60);assert(yard.scene.current()=='pause')
yard.pressed.pause=true;yard.tick(1/60);assert(yard.scene.current()=='play')
local body=yard.bodies[#yard.bodies]
for _,obj in ipairs(fixtures['examples/top-down/assets/yard.tmj'].layers[2].objects)do
    if obj.type=='charge'then body:setPosition(obj.x,obj.y);yard.tick(1/60)end
end
assert(yard.scene.current()=='result');assert(yard.sounds==8)
assert(storage['progress.json'].kairo_save_version==2 and storage['progress.json'].data.best==8)
yard.pressed.accept=true;yard.tick(1/60);assert(yard.scene.current()=='play')
print('3.4 demo Lua checks: eight starters, scene pause/resume, mixer routing calls, bookmarks, Signal Yard 600 ticks + win/save/restart passed (native boundaries doubled)')
