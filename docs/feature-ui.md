# Visible feature access in 3.4

The 3.3 Feature UI update is retained. After rebuilding, look for **3.4 Workflow
Tools** in the editor title; an older `Kairo.exe` does not change when source is
copied. The feature bar remains visible while editing code or sprites.

Fantasy Console (F7) has enable/canvas/palette/scaling plus Save & Run/Restart.
Link (F8) has explicit Host/Join. Replay (F9) has recording/timeline/bookmarks.
Profiler (F10) displays runtime-reported data. Overview (F6) links starters and tools.
Lua UI opens guidance/snippets, not a misleading global UI-enable switch.

New entries open Inspector, Input Mappings, Particles, Mixer and Project Search.
Ctrl/Cmd+Shift+P opens the command palette. The main toolbar selects Development,
Debug, Release or Micro; it takes effect on the next Run/Build.

Use [editor.md](editor.md) for the complete workflow and safety semantics. Native UI
acceptance has not run in the authoring environment. Rebuild both executables with:

```powershell
.\tools\launch-editor.ps1 -LowDisk -Validate
```
