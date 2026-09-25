local menu, play, pause, result = scene.new('menu'), scene.new('play'), scene.new('pause'), scene.new('result')
local state={x=80,y=120,score=0,time=45,best=0,charges={},moving=false}
local tuning={speed=155,goal=8}
local map, body, robot, idle, run, sparks, pickup
local walls={}
local function sound()
    if audio.isAvailable() then audio.play(pickup,{volume=0.55,bus='sfx',pitch=1+state.score*0.035}) end
end
local function new_round()
    state.x,state.y,state.score,state.time=80,120,0,45
    state.charges={}
    for _,object in ipairs(map:getObjects()) do
        if object.type=='charge' then state.charges[#state.charges+1]=prefab.spawn('charge',{x=object.x,y=object.y}) end
    end
    body:setPosition(state.x,state.y);body:setVelocity(0,0)
    sparks:clear();robot:set('idle',true)
end
function game.load()
    physics.setGravity(0,0)
    map=tilemap.load('assets/yard.tmj')
    for _,rect in ipairs(map:getCollisionRects()) do
        local wall=physics.newRectangle('static',rect.x,rect.y,rect.width,rect.height)
        wall:setRotation(rect.rotation);walls[#walls+1]=wall
    end
    body=physics.newRectangle('dynamic',state.x,state.y,12,12)
    local sheet=graphics.loadTexture('assets/robot.png')
    idle=animation.new(sheet,{{x=0,y=0,w=16,h=16,duration=0.3}})
    run=animation.new(sheet,{{x=16,y=0,w=16,h=16,duration=0.1},{x=32,y=0,w=16,h=16,duration=0.1},{x=48,y=0,w=16,h=16,duration=0.1}})
    robot=animator.new();robot:add('idle',idle);robot:add('run',run)
    robot:transition('idle','run',function()return state.moving end)
    robot:transition('run','idle',function()return not state.moving end)
    sparks=particles.load('effects/pickup.particle.toml')
    pickup=audio.loadSound('assets/pickup.wav',{bus='sfx'})
    save.registerMigration(1,2,function(data)data.best=data.best or data.score or 0;return data end)
    local ok,data=pcall(save.readVersioned,'progress.json',2)
    if ok and data then state.best=data.best or 0 elseif not ok then print('Save retained without modification: '..tostring(data)) end
    inspector.expose('Player.speed',tuning,'speed',{writable=true,min=30,max=400})
    inspector.expose('Round.goal',tuning,'goal',{min=1,max=8})
    inspector.expose('Round.score',state,'score')
    inspector.expose('Round.seconds',state,'time')
    replay.register('round',state);replay.register('tuning',tuning)
    scene.switch('menu')
end
function menu:update() if input.pressed('accept') then new_round();scene.switch('play') end end
function menu:draw()
    graphics.clear(0.035,0.055,0.075)
    graphics.print('SIGNAL YARD',40,130,5)
    graphics.print(localization.get('menu.play'),40,230,2)
    graphics.print('Collect 8 charges before time runs out.',40,290,2)
    graphics.print('WASD / left stick. Escape: pause. B: bookmark.',40,350,1)
    graphics.print('Live Inspector: tune Player.speed. F11: bookmark.',40,380,1)
    graphics.print('Best: '..state.best,40,450,2)
end
function play:update(dt)
    state.x,state.y=body:getPosition()
    local x,y=input.axis('move_x'),input.axis('move_y')
    local length=math.sqrt(x*x+y*y);if length>1 then x,y=x/length,y/length end
    state.moving=length>0.05
    body:setVelocity(x*tuning.speed,y*tuning.speed);body:setRotation(0)
    robot:update(dt);sparks:update(dt);map:update(dt)
    state.time=math.max(0,state.time-dt)
    for _,charge in ipairs(state.charges) do
        if not charge.taken and (state.x-charge.x)^2+(state.y-charge.y)^2<20^2 then
            charge.taken=true;state.score=state.score+1
            sparks:setPosition(charge.x,charge.y);sparks:emit(26);sound()
        end
    end
    if input.pressed('mark') then replay.bookmark('Yard '..state.score..'/8','Check the round in the Live Inspector') end
    if input.pressed('pause') then scene.push('pause')
    elseif state.time==0 or state.score>=tuning.goal then
        state.best=math.max(state.best,state.score)
        local ok,err=pcall(save.writeVersioned,'progress.json',2,{best=state.best})
        if not ok then print('Save failed: '..tostring(err)) end
        scene.switch('result')
    end
end
function play:draw()
    graphics.clear(0.035,0.055,0.075)
    graphics.setCamera(0,0,1.5)
    map:draw()
    for _,charge in ipairs(state.charges) do
        if not charge.taken then
            graphics.setColor(0.85,0.75,0.35);graphics.rectangle('fill',charge.x-4,charge.y-4,8,8)
        end
    end
    graphics.setColor(1,1,1);robot:draw(state.x-8,state.y-8);sparks:draw()
    graphics.resetCamera()
    graphics.print('CHARGES '..state.score..'/8    TIME '..math.ceil(state.time),30,20,2)
    -- Native physics overlays use the final drawing camera.
    graphics.setCamera(0,0,1.5)
end
pause.pause_physics=true
function pause:update()
    if input.pressed('pause') then scene.pop()
    elseif input.pressed('accept') then scene.switch('menu') end
end
function pause:draw()
    graphics.resetCamera()
    graphics.setColor(0,0,0,0.75);graphics.rectangle('fill',0,0,960,600)
    graphics.setColor(1,1,1);graphics.print(localization.get('pause.title'),350,230,3)
    graphics.print('Escape: resume    Enter: menu',235,310,2)
end
result.pause_physics=true
function result:update()if input.pressed('accept')then new_round();scene.switch('play')end end
function result:draw()
    graphics.clear(0.035,0.055,0.075)
    graphics.print(state.score>=tuning.goal and 'YARD RESTORED' or 'TIME UP',40,150,4)
    graphics.print('Charges '..state.score..'/8  Best '..state.best,40,250,3)
    graphics.print(localization.get('result.again'),40,350,2)
end
menu.pause_physics=true
for _,item in ipairs({menu,play,pause,result})do scene.register(item)end
function game.replayRestored() sparks:clear();robot:set(state.moving and 'run' or 'idle',true) end
