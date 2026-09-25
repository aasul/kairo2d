# Game localization

Put flat JSON dictionaries in `locales/en.json`, `locales/fr.json`, etc. Values must be strings. Keys may contain dots; they are not interpreted as nested paths.

```json
{"menu.play":"Play", "score":"Score: {value}"}
```

```lua
localization.setFallback("en")
localization.setLanguage("fr")
local caption = localization.get("score", {value=42})
```

Lookup tries the active locale, fallback, then returns the key itself. Missing placeholder values remain `{name}`. Interpolation accepts scalar string/number/boolean values, never functions or tables. Dictionaries are cached; `localization.reload()` clears the cache for next lookup. Limits: 1 MiB source, 10,000 entries, 8192 bytes per translation; identifiers use 1..32 letters/digits/underscore/hyphen.

Live Inspector's language field sends the same setLanguage operation to the selected runtime. It does not change the editor language or rewrite stored settings. Layout does not automatically handle RTL, pluralization, shaping or language-specific fonts. Use a licensed TTF/OTF font supporting your chosen script; bitmap text is basic Latin only. No font files are bundled.
