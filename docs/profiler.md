# Profiling a running game

Start a project with the **Debug** build profile (or configure a trusted
development profile with `profiler=true`). Press F10 in Editor to open Profiler.
The panel shows the latest runtime sample, a short frame-interval history,
simulation and renderer call timings, actual resource counters, and a sortable
Lua callback table. Samples reach Editor at 5 Hz. The same data is available to
Lua through `profiler.stats()`. `profiler.show(true)` draws a compact in-game
overlay; it does not enable callback timing in an unprofiled session.

Scene and node callbacks include their scene, script path and node path. The
table can sort by total, average, maximum, or invocation count. Game callbacks
are attributed to `main.lua`. The runtime records at most 128 distinct
callbacks per frame and telemeters the 32 with highest total time, with omitted
calls counted separately. This bounds work and Link message size. It can miss
an isolated late callback in a very large scene; profile smaller scenes or
instrument a specific callback when that matters. Very long script and node
paths are shortened with a distinguishing hash suffix in telemetry.

`frame_ms` is the frame interval passed to the simulation. `tick_ms` is
simulation CPU wall time; `render_ms` is the renderer call duration. The phase
timings overlap, so do not add them to estimate total frame time. Physics time
measures the Rapier step, and audio time currently measures voice maintenance;
neither includes every call to those systems. Particle and emitter counters
report the live totals tracked by Kairo emitters, and the sprite counter reports
submitted textured quads.
Neither is a count of visible sprites. GPU execution time and GPU memory are
not measured.

The [profiler demo](../examples/profiler-demo/README.md) registers many script
instances and physics bodies for a repeatable manual inspection workflow.
