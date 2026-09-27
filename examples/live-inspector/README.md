# Live inspector demo

Open this project in Kairo Editor with the Debug build profile and run it. Open
Live Inspector, select `Arena/Boss` in the runtime hierarchy, and edit
`property.attack_delay` from 1.25 to 0.8. The attack counter changes pace on
the next update. Compare the runtime and scene values, then use **Apply to
Scene** to persist the tuning value or **Revert Runtime** to restore it. The
node also exposes a read-only health value and a writable position vector.
Page through the 42-node hierarchy with Previous and Next.

Live edits affect the current runtime. **Apply to Scene** updates
`scenes/arena.scene` only for fields marked `persist=true`. Restarting loads
the saved value. Health is not persistable because it represents transient
gameplay state.
