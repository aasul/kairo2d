local tone
function game.load()
    tone=audio.loadSound('assets/tone.wav',{bus='sfx'})
    audio.setBusVolume('music',0.5)
end
function game.keyPressed(key)
    if not audio.isAvailable() then return end
    if key=='1' then audio.play(tone,{bus='music',looping=true,volume=0.2,pitch=0.5})
    elseif key=='2' then audio.play(tone,{bus='sfx',volume=0.8,pan=-0.5})
    elseif key=='3' then audio.play(tone,{bus='ui',volume=0.6,pitch=1.5,pan=0.5})
    elseif key=='space' then audio.stopBus('master') end
end
function game.draw()
    graphics.clear(0.04,0.055,0.08)
    graphics.print('AUDIO MIXER',40,40,3)
    graphics.print('1: looping music  2: SFX left  3: UI right',40,110,2)
    graphics.print('Space: stop all. Open Audio Mixer for volume / mute.',40,160,1)
    local y=260
    for _,bus in ipairs(audio.mixer()) do
        graphics.print(bus.name..'  '..math.floor(bus.volume*100)..'%  '..(bus.muted and 'MUTED' or 'ON'),40,y,2)
        y=y+45
    end
    if not audio.isAvailable() then graphics.print('No output device: playback unavailable.',40,520,2) end
end
