local player={position={240,250},speed=180,health=10,visible=true,mode='normal',tint={0.3,0.85,0.7,1}}
inspector.expose('Player.speed',player,'speed',{writable=true,min=0,max=800})
inspector.expose('Player.health',player,'health',{min=0,max=10})
inspector.expose('Player.position',player,'position',{kind='vector',writable=true,min=0,max=960})
inspector.expose('Player.visible',player,'visible',{writable=true})
inspector.expose('Player.mode',player,'mode',{writable=true,choices={'normal','fast'}})
inspector.expose('Player.tint',player,'tint',{kind='color',writable=true})
replay.register('player',player)
input.bindAxis('move',{negative={'a','left'},positive={'d','right'},gamepad_axis='left_x'})
function game.update(dt)
    local speed=player.speed*(player.mode=='fast' and 2 or 1)
    player.position[1]=math.max(0,math.min(910,player.position[1]+input.axis('move')*speed*dt))
end
function game.draw()
    graphics.clear(0.04,0.06,0.085)
    graphics.print('LIVE INSPECTOR',32,32,3)
    graphics.print('Open Live Inspector. Health is read-only.',32,84,2)
    graphics.print('Pause before editing moving values. A/D: move. F11: bookmark.',32,120,1)
    if player.visible then
        graphics.setColor(table.unpack(player.tint))
        graphics.rectangle('fill',player.position[1],player.position[2],50,50)
    end
    graphics.setColor(1,1,1)
end
