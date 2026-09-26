local ui = {}
local roots, events = {}, {}
local pressed
local widget = {}
widget.__index = widget
local defaultFont
local palette = {
    panel = {0.10, 0.12, 0.16, 0.96}, text = {0.94, 0.96, 1, 1},
    button = {0.19, 0.28, 0.40, 1}, hover = {0.27, 0.40, 0.55, 1},
    track = {0.16, 0.19, 0.23, 1}, fill = {0.40, 0.76, 0.67, 1}
}
local function finite(n) return type(n) == "number" and n == n and math.abs(n) < math.huge end
local function color(c) graphics.setColor(c[1], c[2], c[3], c[4] or 1) end
local function inside(w, x, y) return x >= w.x and y >= w.y and x < w.x + w.w and y < w.y + w.h end
local function make(kind, options)
    options = options or {}
    local w = setmetatable({kind = kind, options = options, children = {}, visible = true, enabled = true,
        text = options.text or "", value = options.value or 0, x = 0, y = 0, w = 0, h = 0}, widget)
    return w
end
local function detachRoot(w)
    for i = #roots, 1, -1 do if roots[i] == w then table.remove(roots, i) end end
end
function widget:add(child)
    assert(self.kind == "panel" and getmetatable(child) == widget, "add expects a panel and a UI widget")
    assert(not child.parent and child ~= self and #self.children < 1024, "widget already has a parent or child limit exceeded")
    local p = self
    while p do assert(p ~= child, "UI parent cycle"); p = p.parent end
    detachRoot(child)
    child.parent = self
    self.children[#self.children + 1] = child
    return child
end
function widget:setText(text) self.text = tostring(text) end
function widget:setValue(value)
    assert(finite(value), "progress must be finite")
    self.value = math.max(0, math.min(1, value))
end
function widget:setVisible(visible) self.visible = not not visible end
function widget:setEnabled(enabled) self.enabled = not not enabled end
function ui.panel(options)
    assert(#roots < 128, "too many root UI panels")
    local w = make("panel", options)
    roots[#roots + 1] = w
    return w
end
function ui.label(text, options) local w = make("label", options); w.text = tostring(text); return w end
function ui.button(text, onClick, options)
    assert(type(onClick) == "function", "button requires an onClick function")
    local w = make("button", options); w.text = tostring(text); w.onClick = onClick; return w
end
function ui.image(texture, options) local w = make("image", options); w.texture = texture; return w end
function ui.progress(value, options) local w = make("progress", options); w:setValue(value); return w end
function ui.setFont(font) defaultFont = font end
function ui.clear() roots = {}; events = {}; pressed = nil end
function ui._cancelPointer() events = {}; pressed = nil end
function ui.remove(w)
    if w.parent then
        local siblings = w.parent.children
        for i, child in ipairs(siblings) do if child == w then table.remove(siblings, i); break end end
        w.parent = nil
    else detachRoot(w) end
end

local function layout(w, x, y, width, depth)
    assert(depth <= 16, "UI tree is too deep")
    local o = w.options
    assert(finite(x) and finite(y), "invalid UI position")
    assert(o.layout == nil or o.layout == "vertical" or o.layout == "horizontal", "unknown UI layout")
    w.x, w.y = x, y
    w.w = o.width or width or 220
    w.h = o.height or (w.kind == "panel" and 120 or 28)
    assert(finite(w.w) and finite(w.h) and w.w >= 0 and w.h >= 0, "invalid UI size")
    local padding, gap = o.padding or 8, o.gap or 6
    assert(finite(padding) and padding >= 0 and finite(gap) and gap >= 0, "invalid UI padding/gap")
    local cx, cy = x + padding, y + padding
    for _, child in ipairs(w.children) do
        if child.visible then
            layout(child, cx, cy, math.max(0, w.w - padding * 2), depth + 1)
            if o.layout == "horizontal" then cx = cx + child.w + gap else cy = cy + child.h + gap end
        end
    end
end
local function layoutRoots()
    local sw, sh = window.getSize()
    for _, root in ipairs(roots) do
        local o = root.options
        local x, y = o.x or 0, o.y or 0
        local w, h = o.width or 220, o.height or 120
        local anchor = o.anchor or "top_left"
        if anchor == "top_right" then x = sw - w - x
        elseif anchor == "bottom_left" then y = sh - h - y
        elseif anchor == "bottom_right" then x, y = sw - w - x, sh - h - y
        elseif anchor == "center" then x, y = (sw - w) / 2 + x, (sh - h) / 2 + y
        else assert(anchor == "top_left", "unknown UI anchor") end
        layout(root, x, y, w, 0)
    end
end
local function hit(w, x, y)
    if not w.visible or not w.enabled or not inside(w, x, y) then return nil end
    for i = #w.children, 1, -1 do local child = hit(w.children[i], x, y); if child then return child end end
    if w.kind == "button" then return w end
end
function ui._pointer(down, x, y, button)
    if button == 1 and #events < 64 then events[#events + 1] = {down = down, x = x, y = y} end
end
function ui._update()
    layoutRoots()
    local pending = events; events = {}
    for _, e in ipairs(pending) do
        local target
        for i = #roots, 1, -1 do target = hit(roots[i], e.x, e.y); if target then break end end
        if e.down then pressed = target
        else
            local clicked = pressed; pressed = nil
            if target and clicked == target then target.onClick(target) end
        end
    end
end
local function text(w)
    color(w.options.color or palette.text)
    if defaultFont then graphics.print(defaultFont, w.text, w.x + 8, w.y + 5)
    else graphics.print(w.text, w.x + 8, w.y + 6, w.options.text_scale or 1) end
end
local function draw(w, mx, my)
    if not w.visible then return end
    if w.kind == "panel" then
        color(w.options.background or palette.panel); graphics.rectangle("fill", w.x, w.y, w.w, w.h)
    elseif w.kind == "button" then
        color(inside(w, mx, my) and w.enabled and palette.hover or palette.button)
        graphics.rectangle("fill", w.x, w.y, w.w, w.h); text(w)
    elseif w.kind == "label" then text(w)
    elseif w.kind == "progress" then
        color(palette.track); graphics.rectangle("fill", w.x, w.y, w.w, w.h)
        color(w.options.color or palette.fill); graphics.rectangle("fill", w.x, w.y, w.w * w.value, w.h)
    elseif w.kind == "image" then
        local width, height = w.texture:getDimensions()
        graphics.setColor(1, 1, 1, 1)
        graphics.draw(w.texture, {x = w.x, y = w.y, scale_x = w.w / width, scale_y = w.h / height})
    end
    for _, child in ipairs(w.children) do draw(child, mx, my) end
end
function ui._draw()
    if #roots == 0 then return end
    layoutRoots()
    graphics.resetCamera(); graphics.resetViewport(); graphics.resetScissor()
    local mx, my = mouse.position()
    for _, root in ipairs(roots) do draw(root, mx, my) end
    graphics.setColor(1, 1, 1, 1)
end
function ui.wantsMouse()
    local x, y = mouse.position()
    for i = #roots, 1, -1 do if roots[i].visible and inside(roots[i], x, y) then return true end end
    return false
end
function ui._covers(x, y)
    layoutRoots()
    for i = #roots, 1, -1 do
        if roots[i].visible and inside(roots[i], x, y) then return true end
    end
    return false
end
return ui
