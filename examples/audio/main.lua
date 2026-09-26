local chime
local loop
local volume = 0.5

function game.load()
    chime = audio.load("assets/chime.wav")
    if audio.isAvailable() then audio.setMasterVolume(volume) end
end

function game.keyPressed(key)
    if key == "escape" then window.close() end
    if not audio.isAvailable() then return end
    if key == "space" then audio.play(chime) end
    if key == "l" then
        if loop then audio.stop(loop) end
        loop = audio.play(chime, {looping = true, volume = 0.7})
    end
    if key == "s" then audio.stopAll() end
    if key == "up" then volume = math.min(1, volume + 0.1) end
    if key == "down" then volume = math.max(0, volume - 0.1) end
    audio.setMasterVolume(volume)
end

function game.draw()
    graphics.clear(0.08, 0.06, 0.12)
    graphics.print("Kairo2D audio", 48, 48, 3)
    graphics.print("Space: chime | L: loop | S: stop all", 48, 130)
    graphics.print("Up / down: master volume | Escape: quit", 48, 165)
    graphics.print(string.format("Master volume: %.1f", volume), 48, 230)
    graphics.print(string.format("Sound duration: %.2f seconds", chime:getDuration()), 48, 265)
    if not audio.isAvailable() then
        graphics.setColor(1, 0.7, 0.4)
        graphics.print("No audio output device (or audio was disabled).", 48, 340)
    elseif loop and audio.isPlaying(loop) then
        graphics.setColor(0.3, 0.9, 0.7)
        graphics.print("Loop playing", 48, 340)
    end
end
