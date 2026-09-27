local collision = {}

-- Return an outward collision normal and penetration depth for a circle/box pair.
function collision.circleBox(cx, cy, radius, box)
    local x = math.max(box.x, math.min(cx, box.x + box.w))
    local y = math.max(box.y, math.min(cy, box.y + box.h))
    local dx, dy = cx - x, cy - y
    local length = math.sqrt(dx * dx + dy * dy)
    if length >= radius then return nil end
    if length > 0 then return dx / length, dy / length, radius - length end

    -- The centre is inside the box: use the nearest face instead of dividing by zero.
    local distances = { cx - box.x, box.x + box.w - cx, cy - box.y, box.y + box.h - cy }
    local normals = { {-1, 0}, {1, 0}, {0, -1}, {0, 1} }
    local nearest = 1
    for i = 2, 4 do
        if distances[i] < distances[nearest] then nearest = i end
    end
    return normals[nearest][1], normals[nearest][2], radius + distances[nearest]
end

function collision.reflect(vx, vy, nx, ny)
    local dot = vx * nx + vy * ny
    if dot >= 0 then return vx, vy end
    return vx - 2 * dot * nx, vy - 2 * dot * ny
end

return collision
