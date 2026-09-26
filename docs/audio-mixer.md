# Audio buses

Kairo exposes `master`, `music`, `sfx`, and `ui`. Non-master buses are lazy native Kira subtracks routed to master. The mixer UI targets the local game or selected Link peer. It changes live values, not persistent project defaults.

```lua
local laser = audio.loadSound("assets/laser.wav", {bus="sfx"})
audio.setBusVolume("music", 0.6, 0.25)
audio.muteBus("ui", true)
-- Call during input/update, not game.load:
if audio.isAvailable() then
    local voice = audio.play(laser, {volume=0.8, bus="sfx", pitch=1.1, pan=-0.3})
end
```

`audio.load` and `loadSound` are aliases. The decoded sound cache is still path-based; separate wrappers may set different default buses without decoding again. `play` can override bus and accepts volume 0..1, looping boolean, pitch ratio 0.125..4 and pan -1 (left) to 1 (right). Pitch uses resampling, changing speed/duration as well as pitch. It is not time-stretching.

`setBusVolume(bus, volume, seconds?)` fades over 0..30 seconds. Muting retains the target volume. `stopBus` stops voices on a selected bus; `master` stops all. Existing voice stop/volume/isPlaying APIs remain. `audio.mixer()` returns names, target volumes, mute flags and active voice counts. Counts are pruned each tick. This is not peak metering; displayed volume is the target, not a sampled intermediate fade gain.

Bus settings work without an output device; playing still reports unavailability. Startup/candidate code may decode sounds and set mix values but may not play sound during `game.load` or `restoreState`. Playback needs user/update events after acceptance. No custom buses, effect chains or streamed music were added. [Demo](../examples/audio-mixer/README.md).
