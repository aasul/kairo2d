local state={x=80,elapsed=0,color={0.4,0.8,0.6}}
replay.register('state',state)
replay.enable(true)
input.bind('mark',{keyboard={'b'},gamepad={'a'}})
function game.update(dt)
    state.elapsed=state.elapsed+dt;state.x=(state.x+140*dt)%900
    if input.pressed('mark') then replay.bookmark('Motion at '..string.format('%.2f',state.elapsed),'Created in game with B') end
end
function game.draw()
    graphics.clear(0.04,0.05,0.08)
    graphics.print('BUG BOOKMARKS',30,30,3)
    graphics.print('B or F11: pin a state. Restore from Replay / Live Inspector.',30,90,1)
    graphics.setColor(table.unpack(state.color));graphics.rectangle('fill',state.x,260,40,40)
    graphics.setColor(1,1,1);graphics.print('Simulation: '..string.format('%.2f',state.elapsed),30,500,2)
end
