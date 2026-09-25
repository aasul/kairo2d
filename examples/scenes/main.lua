local menu, play, pause = scene.new('menu'), scene.new('play'), scene.new('pause')
local shared = scene.shared
shared.x = 40
pause.pause_physics = true
input.bind('accept', {keyboard={'space','enter'},gamepad={'a'}})
input.bind('pause', {keyboard={'escape'},gamepad={'start'}})
function menu:update()
    if input.pressed('accept') then scene.switch('play') end
end
function menu:draw()
    graphics.clear(0.045,0.06,0.09)
    graphics.print('SCENE WORKSHOP',40,70,4)
    graphics.print('Space / Enter: play',40,140,2)
end
function play:update(dt)
    shared.x = (shared.x + dt*110) % 900
    if input.pressed('pause') then scene.push('pause') end
end
function play:draw()
    graphics.clear(0.045,0.06,0.09)
    graphics.setColor(0.4,0.8,0.75)
    graphics.rectangle('fill',shared.x,260,50,50)
    graphics.setColor(1,1,1)
    graphics.print('Escape: push pause overlay',40,40,2)
end
function pause:update()
    if input.pressed('pause') then scene.pop()
    elseif input.pressed('accept') then scene.switch('menu') end
end
function pause:draw()
    graphics.setColor(0,0,0,0.65); graphics.rectangle('fill',0,0,960,600)
    graphics.setColor(1,1,1); graphics.print('PAUSED',360,190,4)
    graphics.print('Escape: resume    Space: menu',210,350,2)
end
scene.register(menu);scene.register(play);scene.register(pause)
function game.load() scene.switch('menu') end
