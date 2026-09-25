# Breakout

Run `kairo run examples/breakout` from the repository root, or choose the Breakout
project template in Kairo. Move with A/D or Left/Right, launch with Space, pause
with P, restart with R, and quit with Escape. Clear all 45 bricks before losing
three balls. The window is intentionally fixed at 960 x 540.

`main.lua` owns game state and rendering; `collision.lua` contains reusable,
headlessly testable circle/rectangle collision and reflection functions. The demo
uses explicit arcade collision rather than forcing Rapier into a kinematic game.
The separate physics example demonstrates Rapier. Audio is optional and is checked
before opening a voice. All code and the original generated PNG/WAV assets are MIT.
