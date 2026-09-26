# Scene Workshop

Space/Enter begins play. WASD or the arrow keys move the player. Escape pushes or pops pause. Space in the pause overlay returns to the menu. The gameplay scene is loaded from `scenes/play.scene`, which Kairo Editor can edit. Its CharacterBody2D player uses a native Collider2D, Sprite and following Camera2D. A CanvasLayer shows native Text nodes. Two instances of `prefabs/marker.prefab` have static colliders and different color overrides. Their scripts set a visual property while the renderer draws the authored nodes. A global `scene_entered` signal demonstrates node-owned subscriptions.

Run with `kairo run examples/scenes` from the repository root, or create the corresponding starter in Kairo.
