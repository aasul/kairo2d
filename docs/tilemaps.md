# Tiled JSON maps

Kairo loads finite **orthogonal Tiled JSON** (`.tmj` or JSON) with array-encoded tile layers, embedded or external JSON `.tsj` tilesets, image tilesheets and basic object layers. It does not load TMX, LDtk, infinite/chunked maps, group/image layers, diagonal/hex rotation flags or collection-of-images tilesets. Horizontal/vertical flips are supported.

```lua
local map = tilemap.load("assets/world.tmj")
function game.update(dt) map:update(dt) end
function game.draw() map:draw() end
```

Existing methods: `getSize`, `getLayers`, `getTile(layer,x,y)` (zero-based tile coordinates), `getObjects`, `setLayerVisible`, `draw(x?,y?)`. Layer names must be valid for requested operations. Draw culls to the camera/view rectangle and preserves layer order, offsets, visibility and opacity. `setLayerOpacity(name,0..1)` provides a runtime override. Layer parallax factors and map parallax origin are respected; they affect rendering, not physics.

`getProperties`, `getLayerProperties(name)`, and `getTileProperties(gid)` return custom property key/value tables. `getObjects()` includes layer, geometry and a Tiled-style property record array; `findObject(name)` returns the first named object's data or nil. Object coordinates/rotation retain Tiled conventions (top-left pixels/degrees). These are data, not automatically instantiated Lua entities.

Animated tiles use tileset frame IDs and durations in milliseconds; call `map:update(dt)` explicitly with 0..1 seconds to advance. No hidden global animation clock. Source image reload still follows the asset manager's rules.

## Explicit collision import

`getCollisionRects(layer?)` returns at most 8192 `{x,y,width,height,rotation}` rectangles. Returned positions are **centers**, rotation is **radians**. By default, it imports nonzero cells on tile layers with custom boolean property `collision=true`, and rectangle objects marked individually or through an object layer. Passing a layer name explicitly selects that layer. Collision tile layers must have parallax 1,1.

```lua
for _,r in ipairs(map:getCollisionRects()) do
    local body = physics.newRectangle("static",r.x,r.y,r.width,r.height)
    body:setRotation(r.rotation)
end
```

Tile-layer collisions are one rectangle per occupied cell, not greedily merged. Tileset `solid` properties are exposed as data but do not automatically decide collisions. Unsupported collision object shapes (ellipse, point, polygon, polyline, text, tile object) fail explicitly rather than pretending to be rectangles. Tiled rectangle rotation is applied about its top-left before computing the center.

`reload()` stages/parses a map and replaces this map handle only on success. It does not rebuild separately created physics bodies or every other handle for the same source. Automatic local/remote reload for these assets is a new session unless game hooks preserve supported state. The editor opens map JSON as source; no internal map painter or dedicated map-preview panel was added.
