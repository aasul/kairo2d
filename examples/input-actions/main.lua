local player={x=420,y=260,speed=240,color=0}
function game.update(dt)
    player.x=math.max(0,math.min(920,player.x+input.axis('move_x')*player.speed*dt))
    player.y=math.max(90,math.min(560,player.y+input.axis('move_y')*player.speed*dt))
    if input.pressed('activate') then player.color=1-player.color end
end
function game.draw()
    graphics.clear(0.05,0.065,0.09)
    graphics.print('INPUT MAPPINGS',30,25,3)
    graphics.print('WASD / arrows / left stick. Space / A / left click changes color.',30,70,1)
    graphics.setColor(0.3+player.color*0.5,0.75,0.65)
    graphics.rectangle('fill',player.x,player.y,40,40)
    graphics.setColor(1,1,1)
end
