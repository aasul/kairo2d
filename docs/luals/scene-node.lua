---@meta

---@class KairoNode
local KairoNode = {}

---Create an ordered child node; cycles, invalid names and stale references are rejected.
---@param type any
---@param name any
function KairoNode:createChild(type, name) end

---Set a serialized node property; supported scene node types read native render, camera, physics and audio properties.
---@param name any
---@param value any
function KairoNode:setProperty(name, value) end

---Explicitly allow a scene-node field in the live inspector. Use property.name for stored properties, or position, rotation, scale, pivot, enabled, or visible. Fields are read-only unless writable=true; persist=true also permits Apply to Scene for authored scenes. Edits are validated against the current value and metadata.
---@param field any
---@param metadata any
function KairoNode:exposeInspector(field, metadata) end

---Read a serialized property or nil; dynamic body velocity is updated after the physics step.
---@param name any
function KairoNode:getProperty(name) end

---Remove one node property; returns whether the property existed.
---@param name any
function KairoNode:removeProperty(name) end

---Move a node under another parent at a 1-based sibling index; cycles are rejected.
---@param parent any
---@param index? any
function KairoNode:reparent(parent, index?) end

---Subscribe to an event emitted by this node. The subscription is removed when the node is destroyed.
---@param event any
---@param callback any
function KairoNode:on(event, callback) end

---Dispatch a node event synchronously, passing all supplied Lua values to subscribers.
---@param event any
---@param ... any
function KairoNode:emit(event, ...) end

---Disconnect a signal token. Returns false if it was already removed.
---@param token any
function KairoNode:off(token) end
