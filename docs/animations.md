# Sprite animation

```lua
local sheet, walk
function game.load()
    sheet = graphics.loadTexture("assets/pilot.png")
    walk = animation.new(sheet, {
        {x=0, y=0, w=16, h=16, duration=0.1},
        {x=16, y=0, w=16, h=16, duration=0.1}
    })
end
function game.update(dt) walk:update(dt) end
function game.draw() graphics.draw(walk, {x=100,y=120,scale_x=3,scale_y=3}) end
```

Each instance has an explicit clock. There is no global implicit animation tick.
Methods: `update(dt)`, `setLooping(bool)`, `setSpeed(multiplier)`, `pause()`,
`resume()`, `restart()`, `getFrame()` (one-based) and `isFinished()`.
Looping defaults true, speed defaults 1 and ranges 0..100. A non-looping clip holds
its last frame. Restart resets time and unpauses. Delta must be finite/nonnegative.
Clips allow 1..4096 positive-size frames contained within the sheet, each duration
between one microsecond and one hour. Large deltas skip cycles without looping over
every elapsed frame.

Metadata format (`pilot.anim.json`):

```json
{
  "texture": "pilot.png",
  "looping": true,
  "frames": [
    {"x": 0, "y": 0, "w": 16, "h": 16, "duration": 0.1},
    {"x": 16, "y": 0, "w": 16, "h": 16, "duration": 0.1}
  ]
}
```

`animation.load("assets/pilot.anim.json")` resolves the image relative to the metadata
file while preventing project-root escape. It creates a new clock and uses the
cached texture. Metadata is not a second GPU resource type. Drawing an animation
uses its current frame rectangle; it replaces any explicit `source` option.

In Kairo, select a PNG and choose **Assets > Animate selected PNG**, or open existing
`.anim.json`. Slice a grid, edit frame rectangles/durations, duplicate/remove,
preview, toggle looping, and save. The editor limits new grid slices to 512 frames.
It does not supply animation events, layers, onion skinning or metadata undo.
Changing metadata on disk locally triggers a full session reload, not preservation
of every animation clock. Store your own clip/time state explicitly when needed.
