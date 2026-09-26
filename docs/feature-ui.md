# Visible feature access in 3.4

After rebuilding, the editor title includes **3.4 Workflow Tools**. The feature bar
stays visible while you edit code or sprites.

Fantasy Console (F7) has enable/canvas/palette/scaling plus Save & Run/Restart.
Link (F8) has explicit Host/Join. Replay (F9) has recording/timeline/bookmarks.
Profiler (F10) displays runtime-reported data. Overview (F6) links starters and tools.
Lua UI opens guidance/snippets, not a misleading global UI-enable switch.

New entries open Inspector, Input Mappings, Particles, Mixer and Project Search.
Ctrl/Cmd+Shift+P opens the command palette. The main toolbar selects Development,
Debug, Release or Micro; it takes effect on the next Run/Build.

See [editor.md](editor.md) for the full workflow. To rebuild both executables on
Windows, run:

```powershell
.\tools\launch-editor.ps1 -LowDisk -Validate
```
