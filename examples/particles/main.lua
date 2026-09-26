local emitter
function game.load() emitter=particles.load('effects/sparks.particle.toml') end
function game.update(dt)
    local x,y=mouse.position();emitter:setPosition(x,y);emitter:update(dt)
end
function game.mousePressed(_,_,button) if button==1 then emitter:emit(60) end end
function game.draw()
    graphics.clear(0.035,0.045,0.065)
    graphics.print('PARTICLE WORKSHOP',30,30,3)
    graphics.print('Move the mouse. Click for a burst. Edit the preset in Particles.',30,82,1)
    emitter:draw()
    graphics.print('Particles: '..emitter:count(),30,540,2)
end
