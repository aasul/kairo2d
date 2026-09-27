# Fonts

```lua
local font
function game.load() font = graphics.loadFont("assets/font.ttf", 24) end
function game.draw()
    local width, height = graphics.measureText(font, "Hello\nKairo")
    graphics.setColor(1, 1, 1)
    graphics.print(font, "Hello\nKairo", 40, 40)
end
```

The fontdue-backed path loads TTF/OpenType fonts that fontdue can parse. `loadFont`
uses a project-relative path and size in pixels, 4..256. Canonical path + size forms
the cache key. There are at most 32 cached font/size pairs, 8192 cached glyphs per
pair, and eight 1024-square atlas pages per pair. Pages also count toward the common
texture byte budget. Font files are limited to 16 MiB.

Glyphs rasterize on first measurement/draw and are reused. Text is positioned by its
layout top-left; glyph bearings may extend beyond its advance bounds. Layout uses
horizontal advances, pair kerning, newline and four-space tabs. `measureText`
returns advance width and line-block height; it may populate glyph atlases.
One call allows at most 4096 characters and 16 KiB of UTF-8 text.

`graphics.releaseFont(font)` releases its atlas pages and invalidates that handle;
call outside drawing. Use current color/camera for text just like sprites. No raw
font/parser/GPU objects are exposed.

This is not a complete text-shaping engine. Complex-script shaping, bidi, font
fallback, colored emoji, automatic wrapping/alignment and every OpenType outline
format are not guaranteed. Use the existing bitmap overload for tiny retro labels:
`graphics.print("Score", x, y, scale)`.

No font binaries are included in the repository. The fonts example accepts a font
you supply and otherwise displays instructions using the built-in bitmap font.
Font rasterization/GPU output has not been run in the authoring environment.
