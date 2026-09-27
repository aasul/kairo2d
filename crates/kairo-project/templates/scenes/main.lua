local menu, play, pause = scene.new('menu'), scene.load('scenes/play.scene'), scene.new('pause')
local shared = scene.shared
shared.visits = shared.visits or 0

-- These JSON prefab instances are children of the same hierarchy edited in Kairo.
local first = prefab.instantiate('marker', play.root, {shade = 0.25})
first.position = {x = 250, y = 280}
local second = prefab.instantiate('marker', play.root, {shade = 0.65})
second.position = {x = 550, y = 280}

pause.pause_physics = true
input.bind('accept', {keyboard={'space','enter'},gamepad={'a'}})
input.bind('pause', {keyboard={'escape'},gamepad={'start'}})
input.bindAxis('move_x', {negative={'a','left'},positive={'d','right'},gamepad_axis='left_x'})
input.bindAxis('move_y', {negative={'w','up'},positive={'s','down'},gamepad_axis='left_y'})

function menu:update()
    if input.pressed('accept') then scene.switch(play) end
end
function menu:draw()
    graphics.clear(0.045,0.06,0.09)
    graphics.print('SCENE WORKSHOP',40,70,4)
    graphics.print('Space / Enter: play',40,140,2)
end

function play:enter()
    shared.visits = shared.visits + 1
    Events.emit('scene_entered', shared.visits)
end
function play:update()
    if input.pressed('pause') then scene.push(pause) end
    play.root:find('Visits'):setProperty('text', 'Visits: ' .. shared.visits)
end
function play:draw()
    graphics.clear(0.045,0.06,0.09)
end

function pause:update()
    if input.pressed('pause') then scene.pop()
    elseif input.pressed('accept') then scene.switch(menu) end
end
function pause:draw()
    graphics.setColor(0,0,0,0.65)
    graphics.rectangle('fill',0,0,960,600)
    graphics.setColor(1,1,1)
    graphics.print('PAUSED',360,190,4)
    graphics.print('Escape: resume    Space: menu',210,350,2)
end

scene.register(menu)
scene.register(play)
scene.register(pause)
function game.load() scene.switch(menu) end
