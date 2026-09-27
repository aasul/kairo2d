---@meta

profiler = {}
debug = {}

---Enable the runtime CPU/counter overlay.
---@param enabled boolean
function profiler.show(enabled) end

---Read frame interval, CPU timings, runtime counters, and bounded per-callback samples. Callback timing is enabled by the Debug build profile.
---@return table
function profiler.stats() end

---Development-only collider outlines and centers; sleeping bodies have a distinct tint.
---@param enabled boolean
function debug.drawPhysics(enabled) end

---Bounded textured-sprite outline overlay, using the queued camera/viewport.
---@param enabled boolean
function debug.drawBounds(enabled) end

---Velocity vectors when physics debug drawing is enabled.
---@param enabled boolean
function debug.drawVelocities(enabled) end

---Development-only crosshairs for up to 256 visible scene node origins.
---@param enabled boolean
function debug.drawOrigins(enabled) end

---Development-only labels for up to 256 visible scene nodes.
---@param enabled boolean
function debug.drawNodeNames(enabled) end

---Development-only camera bounds and center crosshair.
---@param enabled boolean
function debug.drawCamera(enabled) end

---Show/hide the measured CPU/render overlay when profile tools are enabled.
---@param enabled boolean
function debug.showProfiler(enabled) end
