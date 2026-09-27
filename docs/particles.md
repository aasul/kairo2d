# Native particles and the particle editor

Open **Particles** from FEATURES or the command palette. Preview uses the same `kairo-core::particles::Emitter` as the runtime. Play/pause, burst, clear, drag emitter position, numeric controls and RGBA colors operate on real simulated particles. Save/load `.particle.toml`; image paths are relative to that preset.

```lua
local sparks = particles.new({
    rate=0, max_particles=1024, lifetime={0.2,0.6}, speed={80,220},
    direction=-math.pi/2, spread=2.0, gravity=200, drag=0.8,
    size={7,0}, color_start={1,0.8,0.3,1}, color_end={0.7,0.2,0.1,0},
    rotation={0,6.28}, seed=42
})
function game.update(dt) sparks:update(dt) end
function game.draw() sparks:draw() end
-- On an event: sparks:setPosition(x,y); sparks:emit(30)
```

Ranges for lifetime/speed/rotation select random spawn values. `size` is start/end, not a random range; color interpolates over life. Direction and spread are radians, spread centered on direction. Gravity is downward acceleration in pixels/squared-second, drag is exponential. Position changes affect future particles only. `emit(n)` returns the number actually emitted; excess capacity is dropped. `count()` and `clear()` are available.

`texture` in `particles.new` is an existing texture handle. A preset uses optional top-level `texture="../assets/spark.png"` and an `[emitter]` table for the numeric options. `particles.load(path)` loads that preset. Sizes are square; texture tint multiplies the current graphics color. Alpha blending only. Simulation is Rust-side with no per-particle Lua objects.

Limits: 64 emitters per VM, 8192 particles per emitter; finite dt within 0..1 seconds; validated ranges; bounded native vectors. The emitter has its own seeded RNG, independent of Kairo's gameplay RNG. Changing a preset seed on an existing editor emitter does not rewind its RNG; load/new recreates it. Particles are transient and not captured by Replay.

The tool supports conflict-detecting saves and Save All/unsaved-project prompts. Choosing Load explicitly discards edits; New permits a new filename. Create the containing folder first. No node editor, collision particles, mesh particles, additive blending, GPU simulation or automatic asset-import pipeline is claimed. [Demo](../examples/particles/README.md).
