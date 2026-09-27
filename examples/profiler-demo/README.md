# Profiler demo

Run `kairo run examples/profiler-demo --profile debug` from the repository root. In Kairo Editor, open this project, select the Debug build profile, run it, and press F10 to open Profiler. The scene registers 48 enemy script instances and 13 physics bodies. Sort the callback table by total, average, maximum, or call count. Toggle collider drawing from Live Inspector.

The example deliberately does work in each enemy update to make callback timing visible. The profiler samples CPU wall time and telemeters at 5 Hz; results vary by machine and are not a GPU benchmark.
