local actor={x=80,y=420,vx=0,vy=0,speed=210,grounded=false}
local platforms={{x=0,y=540,w=960,h=60},{x=240,y=430,w=150,h=20},{x=460,y=330,w=140,h=20},{x=680,y=230,w=150,h=20}}
local effects
input.bind('jump',{keyboard={'space','w'},gamepad={'a'}})
input.bindAxis('move',{negative={'a','left'},positive={'d','right'},gamepad_axis='left_x'})
inspector.expose('Player.speed',actor,'speed',{writable=true,min=50,max=450})
replay.register('actor',actor)
function game.load()effects=particles.new({rate=0,lifetime={0.1,0.3},speed={20,70},gravity=150,size={4,0}})end
function game.update(dt)
    local previous_bottom=actor.y+28
    actor.vx=input.axis('move')*actor.speed
    if actor.grounded and input.pressed('jump')then actor.vy=-410;effects:setPosition(actor.x+12,actor.y+28);effects:emit(16)end
    actor.vy=actor.vy+1100*dt
    actor.x=math.max(0,math.min(936,actor.x+actor.vx*dt));actor.y=actor.y+actor.vy*dt;actor.grounded=false
    for _,p in ipairs(platforms)do
        if actor.vy>=0 and previous_bottom<=p.y and actor.y+28>=p.y and actor.x+24>p.x and actor.x<p.x+p.w then
            actor.y=p.y-28;actor.vy=0;actor.grounded=true
        end
    end
    if actor.y>650 then actor.y=350;actor.vy=0 end
    effects:update(dt)
end
function game.draw()
    graphics.clear(0.045,0.06,0.09)
    graphics.print('PLATFORMER STARTER',30,30,3)
    graphics.print('A/D or left stick: move. Space/A: jump. F11: bookmark.',30,80,1)
    graphics.setColor(0.2,0.4,0.45)
    for _,p in ipairs(platforms)do graphics.rectangle('fill',p.x,p.y,p.w,p.h)end
    graphics.setColor(0.4,0.85,0.7);graphics.rectangle('fill',actor.x,actor.y,24,28)
    graphics.setColor(1,1,1);effects:draw()
end
function game.replayRestored()effects:clear()end
