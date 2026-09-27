-- Relay Dusk: one complete mission built from public Kairo APIs.
local W, H, WORLD_W, WORLD_H = 1280, 720, 2304, 1536
local TAU = math.pi * 2
local menu, play, pause, upgrade, ending = scene.new('menu'), scene.new('mission'), scene.new('pause'), scene.new('upgrade'), scene.new('ending')
local textures, sounds, impact, music, bed = {}, {}, nil, nil, nil
local walls, decorations = {}, {}
local relays = {{x=388,y=362}, {x=1120,y=1110}, {x=1900,y=385}}
local palette = {
    teal={0.30,0.91,0.81}, amber={1.00,0.70,0.35}, rose={0.98,0.38,0.54},
    lavender={0.76,0.56,1.00}, ink={0.035,0.060,0.095}, mist={0.57,0.70,0.78}
}
local player, run, record, cards
local cam_x, cam_y, zoom = 0, 0, 1
local aim_x, aim_y = 1, 0
local time = 0

input.bind('start', {keyboard={'enter'}, gamepad={'a'}})
input.bind('fire', {keyboard={'space'}, mouse={1}, gamepad={'right_shoulder'}})
input.bind('dash', {keyboard={'lshift','rshift'}, gamepad={'b'}})
input.bind('pause', {keyboard={'escape'}, gamepad={'start'}})
input.bind('card1', {keyboard={'1'}, gamepad={'x'}})
input.bind('card2', {keyboard={'2'}, gamepad={'a'}})
input.bind('card3', {keyboard={'3'}, gamepad={'y'}})
input.bindAxis('move_x', {negative={'a','left'},positive={'d','right'},gamepad_axis='left_x',dead_zone=0.18})
input.bindAxis('move_y', {negative={'w','up'},positive={'s','down'},gamepad_axis='left_y',dead_zone=0.18})
input.bindAxis('aim_x', {gamepad_axis='right_x',dead_zone=0.22})
input.bindAxis('aim_y', {gamepad_axis='right_y',dead_zone=0.22})

local function clamp(x, a, b) return math.max(a, math.min(b, x)) end
local function len(x, y) return math.sqrt(x*x+y*y) end
local function dist2(ax,ay,bx,by) return (ax-bx)^2+(ay-by)^2 end
local function tint(c, a) graphics.setColor(c[1],c[2],c[3],a or 1) end
local function box(x,y,w,h,c,a)
    tint(c,a); graphics.rectangle('fill',x,y,w,h)
end
local function text(value,x,y,scale,c)
    tint(c or {0.88,0.96,0.96}); graphics.print(tostring(value),x,y,scale or 1)
end
local function sound(name, volume, pitch)
    if audio.isAvailable() and sounds[name] then
        audio.play(sounds[name],{bus='sfx',volume=volume or 0.5,pitch=pitch or 1})
    end
end
local function burst(x,y,count)
    impact:setPosition(x,y); impact:emit(count)
end
local function sprite(name,x,y,rotation,scale,alpha)
    local texture = textures[name]
    tint({1,1,1},alpha or 1)
    graphics.draw(texture,{x=x,y=y,rotation=rotation or 0,scale_x=scale or 1,scale_y=scale or 1,
        origin_x=name=='boss' and 48 or name=='core' and 12 or 24,
        origin_y=name=='boss' and 48 or name=='core' and 12 or 24})
end
local function meter(x,y,w,h,fraction,color)
    box(x,y,w,h,{0.13,0.20,0.26})
    box(x+2,y+2,math.max(0,(w-4)*clamp(fraction,0,1)),h-4,color)
end

local function add_wall(x,y,w,h)
    walls[#walls+1]={x=x,y=y,w=w,h=h}
    physics.newRectangle('static',x+w/2,y+h/2,w,h)
end
local function world_setup()
    physics.setGravity(0,0)
    add_wall(-24,-24,WORLD_W+48,32)
    add_wall(-24,WORLD_H-8,WORLD_W+48,32)
    add_wall(-24,0,32,WORLD_H)
    add_wall(WORLD_W-8,0,32,WORLD_H)
    for _,r in ipairs({
        {685,242,76,220},{681,880,76,260},{1485,285,76,260},{1480,1015,76,260},
        {960,575,380,55},{315,735,245,50},{1755,765,240,50},
        {1090,195,130,48},{1085,1290,145,48}
    }) do add_wall(r[1],r[2],r[3],r[4]) end
    for i=1,125 do
        local x=70+random.float()*(WORLD_W-140)
        local y=70+random.float()*(WORLD_H-140)
        decorations[#decorations+1]={x=x,y=y,size=random.integer(2,5),glow=random.float()}
    end
end

local function save_record()
    local ok, err = pcall(save.write,'record.json',record)
    if not ok then print('Relay Dusk: record was not saved: '..tostring(err)) end
end
local function new_run()
    run={phase=1,charge=0,elapsed=0,score=0,kills=0,enemies={},shots={},hostile={},drops={},
        spawn=1.1,clear=false,finished=false,boss_spawned=false,shake=0,flash=0,combo=0,combo_timer=0}
    player={body=player and player.body or physics.newCircle('dynamic',350,422,16),
        x=350,y=422,hp=100,maxhp=100,speed=290,damage=22,rate=0.17,shot_timer=0,
        dash=0,dash_cd=0,invuln=0,multi=1,pierce=0,magnet=84}
    player.body:setPosition(player.x,player.y)
    player.body:setVelocity(0,0)
    aim_x,aim_y=1,0
    cam_x,cam_y=0,0
    impact:clear()
end

local enemy_data={
    hunter={hp=64,speed=128,r=19,damage=13,score=90},
    sniper={hp=45,speed=83,r=18,damage=9,score=140},
    tank={hp=160,speed=59,r=24,damage=20,score=210},
    mote={hp=32,speed=195,r=16,damage=9,score=70},
    boss={hp=1300,speed=76,r=46,damage=25,score=2500}
}
local function spawn_enemy(kind,x,y)
    if #run.enemies>=65 then return end
    local d=enemy_data[kind]
    run.enemies[#run.enemies+1]={kind=kind,x=x,y=y,hp=d.hp,maxhp=d.hp,speed=d.speed,
        r=d.r,damage=d.damage,score=d.score,attack=0.5+random.float(),hit=0,phase=random.float()*TAU}
end
local function spawn_ring()
    local angle=random.float()*TAU
    local radius=430+random.float()*170
    local x=clamp(player.x+math.cos(angle)*radius,80,WORLD_W-80)
    local y=clamp(player.y+math.sin(angle)*radius,80,WORLD_H-80)
    local roll=random.float()
    local kind=roll<0.43 and 'hunter' or roll<0.68 and 'sniper' or roll<0.84 and 'mote' or 'tank'
    spawn_enemy(kind,x,y)
end
local function spawn_hostile(x,y,angle,speed,damage,radius)
    if #run.hostile>=140 then return end
    run.hostile[#run.hostile+1]={x=x,y=y,vx=math.cos(angle)*speed,vy=math.sin(angle)*speed,
        damage=damage,life=4,r=radius or 7}
end
local function kill_enemy(index)
    local e=run.enemies[index]
    burst(e.x,e.y,e.kind=='boss' and 90 or 18)
    sound(e.kind=='boss' and 'boss' or 'hit',e.kind=='boss' and 0.7 or 0.28,0.75)
    run.score=run.score+e.score
    run.kills=run.kills+1
    run.combo=run.combo+1; run.combo_timer=3
    if e.kind=='boss' then run.clear=true; run.hostile={}
    else
        run.drops[#run.drops+1]={x=e.x,y=e.y,kind='core',life=13}
        if random.float()<0.065 then run.drops[#run.drops+1]={x=e.x+13,y=e.y,kind='heal',life=13} end
        if e.kind=='tank' then
            spawn_enemy('mote',e.x-18,e.y); spawn_enemy('mote',e.x+18,e.y)
        end
    end
    table.remove(run.enemies,index)
end
local function segment_hit(x1,y1,x2,y2,cx,cy,r)
    local dx,dy=x2-x1,y2-y1
    local l2=dx*dx+dy*dy
    local t=l2>0 and clamp(((cx-x1)*dx+(cy-y1)*dy)/l2,0,1) or 0
    return dist2(x1+dx*t,y1+dy*t,cx,cy)<=r*r
end
local function fire_shot()
    local count=player.multi
    for i=1,count do
        local angle=math.atan(aim_y,aim_x)+(i-(count+1)/2)*0.115
        local vx,vy=math.cos(angle),math.sin(angle)
        run.shots[#run.shots+1]={x=player.x+vx*25,y=player.y+vy*25,
            vx=vx*940,vy=vy*940,damage=player.damage,life=1.45,pierce=player.pierce}
    end
    sound('shot',0.22,0.95+random.float()*0.16)
end
local function hurt(amount)
    if player.invuln>0 or run.finished then return end
    player.hp=math.max(0,player.hp-amount)
    player.invuln=0.6; run.shake=13; run.flash=0.18
    burst(player.x,player.y,20); sound('hit',0.52,0.72)
end

local upgrades={
    {name='HOTTER CORE',desc='+30% blaster damage',apply=function()player.damage=player.damage*1.3 end},
    {name='FAST CAPACITOR',desc='Fire 22% faster',apply=function()player.rate=math.max(0.075,player.rate*0.78) end},
    {name='SIDE BARRELS',desc='Add another projectile',apply=function()player.multi=math.min(5,player.multi+1) end},
    {name='REINFORCED HULL',desc='+30 maximum hull; repair 45',apply=function()player.maxhp=player.maxhp+30;player.hp=math.min(player.maxhp,player.hp+45) end},
    {name='THRUSTER ARRAY',desc='+18% movement speed',apply=function()player.speed=player.speed*1.18 end},
    {name='PENETRATOR',desc='Shots pierce one more target',apply=function()player.pierce=player.pierce+1 end},
    {name='COLLECTOR FIELD',desc='Wider pickup field; repair 20',apply=function()player.magnet=player.magnet+55;player.hp=math.min(player.maxhp,player.hp+20) end}
}
local function choose_cards()
    local choices={}
    for i=1,#upgrades do choices[i]=i end
    for i=#choices,2,-1 do
        local j=random.integer(1,i)
        choices[i],choices[j]=choices[j],choices[i]
    end
    cards={upgrades[choices[1]],upgrades[choices[2]],upgrades[choices[3]]}
end
local function finish(won)
    if run.finished then return end
    run.finished=true; run.won=won
    if won then sound('victory',0.8) end
    if run.score>record.best then record.best=run.score end
    if won then record.clears=record.clears+1;record.fastest=math.min(record.fastest or run.elapsed,run.elapsed) end
    save_record(); scene.switch(ending)
end

function game.load()
    for _,name in ipairs({'player','hunter','sniper','tank','mote','boss','core'}) do
        textures[name]=graphics.loadTexture('assets/'..name..'.png')
        graphics.setFilter(textures[name],'nearest')
    end
    for _,name in ipairs({'shot','hit','pickup','relay','dash','boss','victory'}) do
        sounds[name]=audio.loadSound('assets/'..name..'.wav',{bus='sfx'})
    end
    impact=particles.load('effects/impact.particle.toml')
    world_setup()
    local ok,saved=pcall(save.read,'record.json')
    record=ok and type(saved)=='table' and saved or {best=0,clears=0}
    record.best=tonumber(record.best) or 0
    record.clears=tonumber(record.clears) or 0
    if audio.isAvailable() then
        bed=audio.loadSound('assets/drone.wav',{bus='music'})
        audio.setBusVolume('music',0.48)
    end
    scene.switch(menu)
end

local function update_aim()
    local ax,ay=input.axis('aim_x'),input.axis('aim_y')
    if ax*ax+ay*ay>0.09 then
        local m=len(ax,ay); aim_x,aim_y=ax/m,ay/m
    else
        local mx,my=mouse.position()
        if mx>0 or my>0 then
            local dx,dy=cam_x+mx/zoom-player.x,cam_y+my/zoom-player.y
            local m=len(dx,dy)
            if m>12 then aim_x,aim_y=dx/m,dy/m end
        end
    end
end
local function update_player(dt)
    player.x,player.y=player.body:getPosition()
    player.invuln=math.max(0,player.invuln-dt)
    player.dash_cd=math.max(0,player.dash_cd-dt)
    player.shot_timer=math.max(0,player.shot_timer-dt)
    local vx,vy=input.axis('move_x'),input.axis('move_y')
    local m=len(vx,vy)
    if m>1 then vx,vy=vx/m,vy/m end
    update_aim()
    if input.pressed('dash') and player.dash_cd==0 then
        local d=m>0.1 and m or 1
        player.dx,player.dy=m>0.1 and vx/d or aim_x,m>0.1 and vy/d or aim_y
        player.dash=0.18;player.dash_cd=1.55;player.invuln=math.max(player.invuln,0.24)
        burst(player.x,player.y,25);sound('dash',0.45)
    end
    if player.dash>0 then
        player.dash=math.max(0,player.dash-dt)
        player.body:setVelocity(player.dx*800,player.dy*800)
        if random.float()<0.6 then burst(player.x,player.y,2) end
    else player.body:setVelocity(vx*player.speed,vy*player.speed) end
    if input.held('fire') and player.shot_timer<=0 then
        fire_shot(); player.shot_timer=player.rate
    end
end
local function update_enemies(dt)
    for i=#run.enemies,1,-1 do
        local e=run.enemies[i]
        e.hit=math.max(0,e.hit-dt);e.attack=e.attack-dt;e.phase=e.phase+dt
        local dx,dy=player.x-e.x,player.y-e.y
        local d=math.max(1,len(dx,dy))
        local nx,ny=dx/d,dy/d
        if e.kind=='sniper' then
            local motion=d>330 and 1 or d<220 and -1 or 0
            e.x=e.x+nx*e.speed*motion*dt;e.y=e.y+ny*e.speed*motion*dt
            if e.attack<=0 and d<600 then
                spawn_hostile(e.x,e.y,math.atan(dy,dx),330,10)
                e.attack=2.1+random.float()*0.35
            end
        elseif e.kind=='boss' then
            if d>190 then e.x=e.x+nx*e.speed*dt;e.y=e.y+ny*e.speed*dt end
            if e.attack<=0 then
                local base=e.phase*0.6
                for n=0,11 do spawn_hostile(e.x,e.y,base+n*TAU/12,280,13,9) end
                for n=-1,1 do spawn_hostile(e.x,e.y,math.atan(dy,dx)+n*0.20,460,15,8) end
                e.attack=e.hp<e.maxhp*0.4 and 1.25 or 1.85
                run.shake=math.max(run.shake,6);sound('boss',0.35,1.2)
            end
        else
            e.x=e.x+nx*e.speed*dt;e.y=e.y+ny*e.speed*dt
        end
        e.x=clamp(e.x,35,WORLD_W-35);e.y=clamp(e.y,35,WORLD_H-35)
        if d<e.r+15 then hurt(e.damage) end
    end
    run.spawn=run.spawn-dt
    if run.spawn<=0 and run.phase<=4 and not run.clear then
        spawn_ring()
        if run.phase>=3 and random.float()<0.35 then spawn_ring() end
        run.spawn=math.max(0.72,2.55-run.phase*0.30-run.elapsed*0.006)
    end
end
local function update_shots(dt)
    for i=#run.shots,1,-1 do
        local p=run.shots[i]
        local ox,oy=p.x,p.y
        p.x,p.y=p.x+p.vx*dt,p.y+p.vy*dt
        p.life=p.life-dt
        local remove=p.life<=0 or p.x<0 or p.x>WORLD_W or p.y<0 or p.y>WORLD_H
        if not remove then
            for j=#run.enemies,1,-1 do
                local e=run.enemies[j]
                if segment_hit(ox,oy,p.x,p.y,e.x,e.y,e.r+5) then
                    e.hp=e.hp-p.damage;e.hit=0.12;burst(p.x,p.y,4)
                    if e.hp<=0 then kill_enemy(j) else sound('hit',0.13,1.35) end
                    if p.pierce<=0 then remove=true;break else p.pierce=p.pierce-1 end
                end
            end
        end
        if remove then table.remove(run.shots,i) end
    end
    for i=#run.hostile,1,-1 do
        local p=run.hostile[i]
        local ox,oy=p.x,p.y
        p.x,p.y=p.x+p.vx*dt,p.y+p.vy*dt;p.life=p.life-dt
        if segment_hit(ox,oy,p.x,p.y,player.x,player.y,p.r+14) then
            hurt(p.damage);table.remove(run.hostile,i)
        elseif p.life<=0 or p.x<0 or p.x>WORLD_W or p.y<0 or p.y>WORLD_H then
            table.remove(run.hostile,i)
        end
    end
end
local function update_drops(dt)
    for i=#run.drops,1,-1 do
        local d=run.drops[i]
        d.life=d.life-dt
        local dx,dy=player.x-d.x,player.y-d.y
        local m=len(dx,dy)
        if m<player.magnet and m>1 then
            local speed=190+(player.magnet-m)*3
            d.x=d.x+dx/m*speed*dt;d.y=d.y+dy/m*speed*dt
        end
        if m<23 then
            if d.kind=='heal' then player.hp=math.min(player.maxhp,player.hp+18)
            else run.score=run.score+25 end
            sound('pickup',0.22,1.25);burst(d.x,d.y,7)
            table.remove(run.drops,i)
        elseif d.life<=0 then table.remove(run.drops,i) end
    end
end
local function update_objective(dt)
    if run.phase<=3 then
        local r=relays[run.phase]
        local near=dist2(player.x,player.y,r.x,r.y)<100^2
        local contested=false
        for _,e in ipairs(run.enemies) do
            if dist2(e.x,e.y,r.x,r.y)<115^2 then contested=true;break end
        end
        if near and not contested then run.charge=math.min(1,run.charge+dt/5.2)
        else run.charge=math.max(0,run.charge-dt/13) end
        if run.charge>=1 then
            burst(r.x,r.y,70);sound('relay',0.75)
            run.score=run.score+550;run.phase=run.phase+1;run.charge=0
            player.hp=math.min(player.maxhp,player.hp+20)
            choose_cards();scene.push(upgrade)
        end
    elseif not run.boss_spawned then
        run.boss_spawned=true
        spawn_enemy('boss',1960,1050)
        sound('boss',0.75)
    elseif run.clear and dist2(player.x,player.y,1960,1050)<75^2 then
        finish(true)
    end
end

menu.pause_physics=true
function menu:update(dt)
    if bed and not music then music=audio.play(bed,{bus='music',volume=0.5,looping=true}) end
    time=time+dt
    if input.pressed('start') or input.pressed('fire') then new_run();scene.switch(play) end
end
function play:update(dt)
    time=time+dt;run.elapsed=run.elapsed+dt
    if input.pressed('pause') then scene.push(pause);return end
    update_player(dt)
    update_enemies(dt)
    update_shots(dt)
    update_drops(dt)
    update_objective(dt)
    impact:update(dt)
    run.shake=math.max(0,run.shake-dt*44)
    run.flash=math.max(0,run.flash-dt)
    run.combo_timer=math.max(0,run.combo_timer-dt)
    if run.combo_timer==0 then run.combo=0 end
    cam_x=clamp(player.x-W/(2*zoom),0,WORLD_W-W/zoom)
    cam_y=clamp(player.y-H/(2*zoom),0,WORLD_H-H/zoom)
    if player.hp<=0 then finish(false) end
end
pause.pause_physics=true
function pause:update()
    if input.pressed('pause') or input.pressed('start') then scene.pop() end
end
upgrade.pause_physics=true
function upgrade:update()
    local choice=input.pressed('card1') and 1 or input.pressed('card2') and 2 or input.pressed('card3') and 3
    if choice then
        cards[choice].apply();sound('pickup',0.55,0.8+choice*0.15)
        scene.pop()
    end
end
ending.pause_physics=true
function ending:update()
    if input.pressed('start') or input.pressed('fire') then scene.switch(menu) end
end

local function draw_floor()
    graphics.clear(palette.ink[1],palette.ink[2],palette.ink[3])
    local x0=math.floor(cam_x/64)*64
    local y0=math.floor(cam_y/64)*64
    for x=x0,math.min(WORLD_W,cam_x+W)+64,64 do box(x,cam_y,1,H,{0.075,0.14,0.19}) end
    for y=y0,math.min(WORLD_H,cam_y+H)+64,64 do box(cam_x,y,W,1,{0.075,0.14,0.19}) end
    for _,d in ipairs(decorations) do
        if d.x>cam_x-10 and d.x<cam_x+W+10 and d.y>cam_y-10 and d.y<cam_y+H+10 then
            box(d.x,d.y,d.size,d.size,{0.15,0.34,0.40},0.25+d.glow*0.35)
        end
    end
    for _,w in ipairs(walls) do
        box(w.x+5,w.y+7,w.w,w.h,{0,0,0},0.45)
        box(w.x,w.y,w.w,w.h,{0.13,0.25,0.32})
        box(w.x,w.y,w.w,5,{0.30,0.51,0.58})
        box(w.x+5,w.y+8,w.w-10,2,{0.08,0.17,0.23})
    end
end
local function draw_relay(r,index)
    local active=index==run.phase
    local done=index<run.phase
    local c=done and palette.teal or active and palette.amber or {0.25,0.37,0.43}
    local pulse=math.sin(time*3+index)*3
    box(r.x-57,r.y-57,114,114,{0.04,0.12,0.17})
    box(r.x-47,r.y-47,94,94,c,0.15)
    graphics.setColor(c[1],c[2],c[3],0.55)
    graphics.rectangle('line',r.x-47-pulse,r.y-47-pulse,94+pulse*2,94+pulse*2,3)
    box(r.x-17,r.y-17,34,34,c,0.9)
    box(r.x-7,r.y-7,14,14,{0.95,1,0.93})
    if active then meter(r.x-53,r.y+67,106,10,run.charge,palette.amber) end
end
local function draw_world()
    local sx,sy=0,0
    if run.shake>0 then sx=(random.float()-0.5)*run.shake;sy=(random.float()-0.5)*run.shake end
    graphics.setCamera(cam_x+sx,cam_y+sy,zoom)
    draw_floor()
    for i,r in ipairs(relays) do draw_relay(r,i) end
    if run.clear then
        local p=0.5+0.5*math.sin(time*5)
        box(1892,982,136,136,palette.teal,0.08+p*0.12)
        graphics.setColor(0.4,1,0.86,0.7)
        graphics.rectangle('line',1902,992,116,116,4)
        text('EXIT',1923,1032,2,palette.teal)
    end
    for _,d in ipairs(run.drops) do
        if d.kind=='heal' then
            box(d.x-9,d.y-9,18,18,{0.18,0.63,0.44})
            box(d.x-2,d.y-7,4,14,{0.95,1,0.9})
            box(d.x-7,d.y-2,14,4,{0.95,1,0.9})
        else sprite('core',d.x,d.y,time*1.8,1) end
    end
    for _,p in ipairs(run.shots) do
        box(p.x-9,p.y-3,18,6,palette.teal,0.32)
        box(p.x-4,p.y-4,8,8,{0.86,1,0.93})
    end
    for _,p in ipairs(run.hostile) do
        box(p.x-7,p.y-7,14,14,palette.rose,0.48)
        box(p.x-3,p.y-3,6,6,{1,0.86,0.88})
    end
    for _,e in ipairs(run.enemies) do
        local scale=e.kind=='boss' and 1.18 or 0.90
        sprite(e.kind,e.x,e.y,e.kind=='mote' and time*2 or 0,scale,e.hit>0 and 0.65 or 1)
        if e.kind=='boss' then
            meter(e.x-76,e.y-72,152,10,e.hp/e.maxhp,palette.lavender)
        elseif e.hit>0 then meter(e.x-20,e.y-31,40,5,e.hp/e.maxhp,palette.rose) end
    end
    sprite('player',player.x,player.y,math.atan(aim_y,aim_x)+math.pi/2,0.95,
        player.invuln>0 and (math.floor(time*20)%2==0 and 0.45 or 1) or 1)
    impact:draw()
    graphics.resetCamera()
end
local function draw_hud()
    box(0,0,W,92,{0.025,0.045,0.072},0.96)
    box(0,90,W,2,palette.teal,0.36)
    text('RELAY DUSK',24,16,3,palette.teal)
    text('HULL',25,58,1,palette.mist)
    meter(83,57,260,16,player.hp/player.maxhp,player.hp<30 and palette.rose or palette.teal)
    text(math.ceil(player.hp)..'/'..player.maxhp,353,57,1)
    local objective=run.clear and 'REACH THE EXTRACTION GATE' or
        run.phase==4 and 'DESTROY THE WARDEN' or 'ACTIVATE RELAY '..run.phase..' / 3'
    text(objective,467,21,2,run.clear and palette.teal or palette.amber)
    text('SCORE '..run.score,470,59,1,palette.mist)
    text('KILLS '..run.kills,660,59,1,palette.mist)
    text(string.format('%02d:%02d',math.floor(run.elapsed/60),math.floor(run.elapsed%60)),805,59,1,palette.mist)
    text('DASH',1088,16,1,palette.mist)
    meter(1088,38,160,13,1-player.dash_cd/1.55,palette.teal)
    text(player.dash_cd<=0 and 'READY' or 'RECHARGING',1088,58,1,player.dash_cd<=0 and palette.teal or palette.mist)
    box(19,H-80,265,60,{0.035,0.07,0.105},0.88)
    local map_w,map_h=230,42
    for i,r in ipairs(relays) do
        local x=35+r.x/WORLD_W*map_w
        local y=H-71+r.y/WORLD_H*map_h
        box(x-3,y-3,7,7,i<run.phase and palette.teal or i==run.phase and palette.amber or palette.mist)
    end
    box(35+player.x/WORLD_W*map_w-3,H-71+player.y/WORLD_H*map_h-3,7,7,{1,1,1})
    box(994,H-47,271,30,{0.035,0.07,0.105},0.88)
    text('WASD MOVE  SHIFT DASH  ESC PAUSE',1004,H-39,1,palette.mist)
    if run.combo>=3 and run.combo_timer>0 then text('CHAIN x'..run.combo,495,108,2,palette.amber) end
    if run.flash>0 then box(0,0,W,H,palette.rose,run.flash*1.8) end
end
function play:draw()
    draw_world();draw_hud()
end

local function backdrop()
    graphics.clear(0.025,0.045,0.072)
    for x=0,W,64 do box(x,0,1,H,{0.07,0.13,0.19}) end
    for y=0,H,64 do box(0,y,W,1,{0.07,0.13,0.19}) end
    box(0,0,13,H,palette.teal,0.65)
end
function menu:draw()
    backdrop()
    box(60,72,1120,574,{0.035,0.075,0.11},0.93)
    box(60,72,1120,5,palette.teal)
    box(83,110,8,73,palette.amber)
    text('RELAY DUSK',111,107,6,palette.teal)
    text('A tactical survival mission across a collapsing signal grid.',113,180,2,palette.mist)
    sprite('player',933,330,-0.8,4)
    box(110,251,565,2,palette.teal,0.5)
    text('THE MISSION',111,273,2,palette.amber)
    text('Charge three relays while holding back the swarm.',111,321,2)
    text('Choose a system upgrade after each relay.',111,358,2)
    text('Defeat the Warden and reach extraction.',111,395,2)
    box(105,474,488,70,{0.10,0.32,0.35})
    box(105,474,7,70,palette.teal)
    text('PRESS ENTER TO DEPLOY',132,494,3,{0.94,1,0.95})
    text('WASD / stick: move    Mouse / right stick: aim',111,567,1,palette.mist)
    text('Click / Space / RB: fire    Shift / B: dash    Esc: pause',111,590,1,palette.mist)
    text('BEST '..record.best..'     CLEARS '..record.clears,770,584,2,palette.amber)
end
function pause:draw()
    graphics.resetCamera()
    box(0,0,W,H,{0.01,0.025,0.04},0.78)
    box(365,190,550,330,{0.055,0.12,0.16},0.97)
    box(365,190,550,5,palette.teal)
    text('MISSION PAUSED',423,244,4,palette.teal)
    text('Escape / Start to resume',447,346,2)
    text('Stay near a relay to charge it.',431,402,2,palette.mist)
end
function upgrade:draw()
    graphics.resetCamera()
    box(0,0,W,H,{0.01,0.025,0.04},0.84)
    text('RELAY ONLINE',390,114,5,palette.teal)
    text('CHOOSE ONE SYSTEM UPGRADE',390,177,2,palette.mist)
    for i=1,3 do
        local x=91+(i-1)*374
        box(x,252,346,286,{0.07,0.15,0.20})
        box(x,252,346,5,i==1 and palette.teal or i==2 and palette.amber or palette.lavender)
        box(x+21,276,41,41,{0.14,0.29,0.33})
        text(tostring(i),x+33,282,2,palette.teal)
        text(cards[i].name,x+22,354,2)
        text(cards[i].desc,x+22,408,1,palette.mist)
        text('PRESS '..i,x+22,492,2,palette.amber)
    end
    text('Controller: X / A / Y',506,583,1,palette.mist)
end
function ending:draw()
    backdrop()
    box(186,117,908,477,{0.05,0.10,0.14})
    box(186,117,908,6,run.won and palette.teal or palette.rose)
    text(run.won and 'SIGNAL RESTORED' or 'SIGNAL LOST',259,171,5,run.won and palette.teal or palette.rose)
    text(run.won and 'The grid is yours. Extraction complete.' or 'The relay network fell silent.',311,274,2)
    text('SCORE  '..run.score,310,348,3,palette.amber)
    text('KILLS  '..run.kills,310,405,2)
    text('TIME   '..string.format('%02d:%02d',math.floor(run.elapsed/60),math.floor(run.elapsed%60)),625,405,2)
    text('BEST   '..record.best,625,348,3,palette.amber)
    text('PRESS ENTER TO RETURN',394,516,2,palette.teal)
end

for _,s in ipairs({menu,play,pause,upgrade,ending}) do scene.register(s) end
