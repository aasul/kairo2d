use kairo_core::{Config, DrawCommand, ProjectFs};
use kairo_lua::GameSession;

fn project(source: &str) -> (tempfile::TempDir, GameSession) {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("main.lua"), source).unwrap();
    std::fs::write(
        root.path().join("pixel.png"),
        include_bytes!("fixtures/pixel.png"),
    )
    .unwrap();
    let session = GameSession::load(
        ProjectFs::new(root.path()).unwrap(),
        &Config::default(),
        false,
    )
    .unwrap();
    (root, session)
}

#[test]
fn lua_scene_graph_preserves_legacy_scene_switch_and_validates_nodes() {
    let (_root, session) = project(
        r#"
        local level = scene.new('Level')
        local world = level:createNode('Node2D', 'World')
        local player = world:createChild('Sprite', 'Player')
        world.position = {x=10, y=20}
        player.position = {x=3, y=4}
        player:addTag('hero')
        assert(player.globalPosition.x == 13 and player.globalPosition.y == 24)
        assert(level.graph:findPath('Level/World/Player').id == player.id)
        assert(#level.graph:findByTag('hero') == 1)
        local copy = player:duplicate()
        assert(copy.id ~= player.id and copy:path() == player:path())
        player:destroy()
        assert(not pcall(function() return player.name end))
        local restored = sceneGraph.fromJson(level.graph:toJson())
        assert(restored:findPath('Level/World/Player') ~= nil)
        scene.switch(level)
        assert(not pcall(scene.switch, scene.new('Level')))
    "#,
    );
    session.tick(1.0 / 60.0).unwrap();
    assert_eq!(
        session
            .lua()
            .globals()
            .get::<mlua::Table>("scene")
            .unwrap()
            .get::<mlua::Function>("current")
            .unwrap()
            .call::<String>(())
            .unwrap(),
        "Level"
    );
}

#[test]
fn authored_scene_draws_native_sprites_camera_and_screen_ui_in_tree_order() {
    let (_root, session) = project(
        r#"
        local level = scene.new('Native')
        local world = level:createNode('Node2D', 'World')
        world.position = {x=20,y=30}
        world.rotation = math.pi / 2
        world.scale = {x=2,y=1}
        local image = world:createChild('Sprite', 'Image')
        image.position = {x=4,y=0}
        image:setProperty('texture', 'pixel.png')
        image:setProperty('source', {x=0,y=0,width=1,height=2})
        image:setProperty('width', 12)
        image:setProperty('flip_x', true)
        image:setProperty('tint', {0.4,0.5,0.6,0.7})
        local hidden = world:createChild('Sprite', 'Hidden')
        hidden:setProperty('texture', 'pixel.png')
        hidden.visible = false
        local cam = level:createNode('Camera2D','Camera')
        cam.position = {x=100,y=80}
        cam:setProperty('active', true)
        cam:setProperty('zoom', 2)
        local canvas = level:createNode('CanvasLayer', 'UI')
        local panel = canvas:createChild('Control','Panel')
        panel.position = {x=8,y=9}
        panel:setProperty('width', 50)
        panel:setProperty('height', 20)
        panel:setProperty('color', {1,0,0,0.5})
        local label = canvas:createChild('Text','Label')
        label.position = {x=10,y=10}
        label:setProperty('text', 'Ready')
        scene.switch(level)
    "#,
    );
    session.tick(1.0 / 60.0).unwrap();
    let state = session.state().borrow();
    assert_eq!(state.assets.texture_count(), 1);
    assert_eq!(state.frame.commands.len(), 3);
    let DrawCommand::AffineQuad { quad, matrix } = &state.frame.commands[0] else {
        panic!("scene sprite should use native affine geometry")
    };
    assert!(quad.texture.is_some());
    assert_eq!(quad.size.to_array(), [12.0, 2.0]);
    assert_eq!(quad.uv_min.to_array(), [0.5, 0.0]);
    assert_eq!(quad.uv_max.to_array(), [0.0, 1.0]);
    assert_eq!(quad.color.0, [0.4, 0.5, 0.6, 0.7]);
    let world = matrix.transform_point2(glam::Vec2::ZERO);
    assert!((world.x - 20.0).abs() < 0.001 && (world.y - 38.0).abs() < 0.001);
    let screen = quad.camera.world_to_screen(glam::Vec2::new(100.0, 80.0));
    assert_eq!(
        screen.to_array(),
        [state.width as f32 / 2.0, state.height as f32 / 2.0]
    );
    let DrawCommand::AffineQuad { quad, .. } = &state.frame.commands[1] else {
        panic!("expected screen-space control")
    };
    assert_eq!(quad.camera.position.to_array(), [0.0, 0.0]);
    assert_eq!(quad.color.0, [1.0, 0.0, 0.0, 0.5]);
    assert!(matches!(&state.frame.commands[2], DrawCommand::Text { text, .. } if text == "Ready"));
    drop(state);
    session.lua().load("local x,y=graphics.worldToScreen(100,80); local wx,wy=graphics.screenToWorld(x,y); assert(math.abs(wx-100)<0.001 and math.abs(wy-80)<0.001)").exec().unwrap();
}

#[test]
fn malformed_authored_sprite_reports_scene_and_node() {
    let (_root, session) = project(
        r#"
        local level = scene.new('Fault')
        local sprite = level:createNode('Sprite','Bad')
        sprite:setProperty('texture', 'pixel.png')
        sprite:setProperty('source', {x=1,y=0,width=2,height=2})
        scene.switch(level)
    "#,
    );
    let error = format!("{:#}", session.tick(0.016).unwrap_err());
    assert!(error.contains("Fault/Bad"), "{error}");
    assert!(
        error.contains("sprite source is outside texture"),
        "{error}"
    );
}

#[test]
fn scene_bodies_sync_rapier_motion_and_release_on_switch_or_destroy() {
    let (_root, session) = project(
        r#"
        level = scene.new('Physics')
        body = level:createNode('CharacterBody2D','Player')
        body.position = {x=100,y=80}
        body:setProperty('velocity', {x=60,y=0})
        local collider = body:createChild('Collider2D','Shape')
        collider:setProperty('shape','rectangle')
        collider:setProperty('width',20)
        collider:setProperty('height',30)
        empty = scene.new('Empty')
        scene.switch(level)
    "#,
    );
    for _ in 0..60 {
        session.tick(1.0 / 60.0).unwrap();
    }
    assert_eq!(session.state().borrow().physics.body_count(), 1);
    session
        .lua()
        .load("assert(body.position.x > 155 and body.position.x < 165); assert(math.abs(body.position.y - 80) < 0.01)")
        .exec()
        .unwrap();
    session
        .lua()
        .load("body:setProperty('velocity', {x=0,y=0}); body.position={x=200,y=50}")
        .exec()
        .unwrap();
    session.tick(1.0 / 60.0).unwrap();
    session
        .lua()
        .load("assert(math.abs(body.position.x - 200) < 0.01)")
        .exec()
        .unwrap();
    session.lua().load("scene.switch(empty)").exec().unwrap();
    session.tick(1.0 / 60.0).unwrap();
    assert_eq!(session.state().borrow().physics.body_count(), 0);
    session.lua().load("scene.switch(level)").exec().unwrap();
    session.tick(1.0 / 60.0).unwrap();
    assert_eq!(session.state().borrow().physics.body_count(), 1);
    session.lua().load("body:destroy()").exec().unwrap();
    session.tick(1.0 / 60.0).unwrap();
    assert_eq!(session.state().borrow().physics.body_count(), 0);
}

#[test]
fn native_body_contacts_emit_validated_scene_node_signals() {
    let (_root, session) = project(
        r#"
        contacts = 0
        local level = scene.new('Contacts')
        local wall = level:createNode('StaticBody2D','Wall')
        wall.position = {x=100,y=100}
        wall:addTag('wall')
        local wallShape = wall:createChild('Collider2D','Shape')
        wallShape:setProperty('width',40)
        wallShape:setProperty('height',40)
        local ball = level:createNode('DynamicBody2D','Ball')
        ball.position = {x=100,y=100}
        local ballShape = ball:createChild('Collider2D','Shape')
        ballShape:setProperty('shape','circle')
        ballShape:setProperty('radius',10)
        ball:on('collision', function(other)
            assert(other:isAlive() and other:hasTag('wall'))
            contacts = contacts + 1
        end)
        scene.switch(level)
    "#,
    );
    session.tick(1.0 / 60.0).unwrap();
    assert!(session.lua().globals().get::<u32>("contacts").unwrap() > 0);
}

#[test]
fn malformed_scene_collider_reports_node_context() {
    let (_root, session) = project(
        r#"
        local level = scene.new('PhysicsFault')
        local body = level:createNode('DynamicBody2D','Player')
        local collider = body:createChild('Collider2D','Shape')
        collider:setProperty('shape','circle')
        collider:setProperty('radius',0)
        scene.switch(level)
    "#,
    );
    let error = format!("{:#}", session.tick(1.0 / 60.0).unwrap_err());
    assert!(error.contains("PhysicsFault/Player"), "{error}");
    assert!(error.contains("positive radius"), "{error}");
}

#[test]
fn scene_audio_source_loads_existing_sound_without_output_device() {
    let (root, session) = project(
        r#"
        local level = scene.new('SoundLevel')
        source = level:createNode('AudioSource','Music')
        source:setProperty('sound','tone.wav')
        source:setProperty('bus','music')
        source:setProperty('looping',true)
        source:setProperty('volume',0.4)
        source:setProperty('autoplay',true)
        scene.switch(level)
    "#,
    );
    std::fs::write(
        root.path().join("tone.wav"),
        include_bytes!("../../kairo-project/templates/audio-mixer/assets/tone.wav"),
    )
    .unwrap();
    session.tick(1.0 / 60.0).unwrap();
    assert_eq!(session.state().borrow().audio.sound_count(), 1);
    assert_eq!(session.state().borrow().audio.voice_count(), 0);
    session
        .lua()
        .load("source:setProperty('autoplay',false)")
        .exec()
        .unwrap();
    session.tick(1.0 / 60.0).unwrap();
}

#[test]
fn authored_controls_hit_test_topmost_and_cancel_pointer_safely() {
    let (_root, session) = project(
        r#"
        clicks = 0
        local level = scene.new('Menu')
        local uiLayer = level:createNode('CanvasLayer', 'UI')
        local back = uiLayer:createChild('Control', 'Back')
        back.position = {x=10,y=10}
        back:setProperty('width', 100)
        back:setProperty('height', 60)
        back:setProperty('interactive', true)
        back:on('click', function() clicks = clicks + 100 end)
        local front = uiLayer:createChild('Control', 'Front')
        front.position = {x=20,y=20}
        front:setProperty('width', 40)
        front:setProperty('height', 40)
        front:setProperty('interactive', true)
        front:on('click', function() clicks = clicks + 1 end)
        scene.switch(level)
    "#,
    );
    session.tick(1.0 / 60.0).unwrap();
    session.ui_pointer(true, 30.0, 30.0, 1).unwrap();
    session.ui_pointer(false, 30.0, 30.0, 1).unwrap();
    assert_eq!(session.lua().globals().get::<u32>("clicks").unwrap(), 1);
    session.ui_pointer(true, 30.0, 30.0, 1).unwrap();
    session.ui_cancel_pointer().unwrap();
    session.ui_pointer(false, 30.0, 30.0, 1).unwrap();
    assert_eq!(session.lua().globals().get::<u32>("clicks").unwrap(), 1);
    session.ui_pointer(true, 70.0, 30.0, 1).unwrap();
    session.ui_pointer(false, 70.0, 30.0, 1).unwrap();
    assert_eq!(session.lua().globals().get::<u32>("clicks").unwrap(), 101);
    session
        .lua()
        .load("ui.panel({x=0,y=0,width=100,height=100})")
        .exec()
        .unwrap();
    session.ui_pointer(true, 30.0, 30.0, 1).unwrap();
    session.ui_pointer(false, 30.0, 30.0, 1).unwrap();
    assert_eq!(session.lua().globals().get::<u32>("clicks").unwrap(), 101);
}

#[test]
fn missing_authored_sound_reports_source_node() {
    let (_root, session) = project(
        r#"
        local level = scene.new('NoSound')
        local source = level:createNode('AudioSource','Music')
        source:setProperty('sound','missing.wav')
        source:setProperty('autoplay',true)
        scene.switch(level)
    "#,
    );
    let error = format!("{:#}", session.tick(1.0 / 60.0).unwrap_err());
    assert!(error.contains("NoSound/Music"), "{error}");
    assert!(error.contains("missing.wav"), "{error}");
}

#[test]
fn node_scripts_have_isolated_state_and_dispatch_only_registered_callbacks() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("scripts")).unwrap();
    std::fs::write(
        root.path().join("scripts/player.lua"),
        r#"
        local Player = {}
        function Player.ready(self) self.health = 100 end
        function Player.update(self, dt) self.health = self.health + dt end
        function Player.onDestroy(self) self.destroyed = true end
        return Player
    "#,
    )
    .unwrap();
    std::fs::write(
        root.path().join("main.lua"),
        r#"
        local level = scene.new('Level')
        local a = level:createNode('Node2D', 'A')
        local b = level:createNode('Node2D', 'B')
        aState = level:attachScript(a, 'scripts/player.lua')
        bState = level:attachScript(b, 'scripts/player.lua')
        scene.switch(level)
    "#,
    )
    .unwrap();
    let session = GameSession::load(
        ProjectFs::new(root.path()).unwrap(),
        &Config::default(),
        false,
    )
    .unwrap();
    session.lua().load("assert(aState ~= bState and aState.health == 100 and bState.health == 100); aState.health = 50").exec().unwrap();
    session.tick(0.1).unwrap();
    session.lua().load("assert(aState.health > 50 and aState.health < 51 and bState.health > 100 and bState.health < 101)").exec().unwrap();
    session.lua().load("aState.node:destroy()").exec().unwrap();
    session.tick(0.1).unwrap();
    session
        .lua()
        .load("assert(aState.destroyed and not bState.destroyed)")
        .exec()
        .unwrap();
}

#[test]
fn node_and_global_signals_disconnect_on_destruction_and_allow_mutation_during_dispatch() {
    let (_root, session) = project(
        r#"
        local level = scene.new('Signals')
        local parent = level:createNode('Node2D', 'Parent')
        local child = parent:createChild('Node2D', 'Child')
        local calls, late, globalCalls = 0, 0, 0
        local second, added
        local first = child:on('damage', function(amount, label)
            assert(amount == 4 and label == 'fire')
            calls = calls + 1
            if not added then
                assert(child:off(second))
                added = child:on('damage', function() late = late + 1 end)
            end
        end)
        second = child:on('damage', function() error('removed during dispatch') end)
        local owned = Events.on('round', function() globalCalls = globalCalls + 1 end, child)
        local unowned = Events.on('round', function() globalCalls = globalCalls + 10 end)
        child:emit('damage', 4, 'fire')
        assert(calls == 1 and late == 0)
        child:emit('damage', 4, 'fire')
        assert(calls == 2 and late == 1)
        Events.emit('round')
        assert(globalCalls == 11)
        parent:destroy()
        assert(not child:isAlive())
        assert(not Events.off(first) and not Events.off(added) and not Events.off(owned))
        assert(not pcall(function() child:emit('damage', 4, 'fire') end))
        Events.emit('round')
        assert(globalCalls == 21 and Events.off(unowned))
    "#,
    );
    session.tick(1.0 / 60.0).unwrap();
}

#[test]
fn scene_workshop_runs_authored_scene_scripts_and_prefab_instances() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/scenes");
    let fs = ProjectFs::new(root).unwrap();
    let config = Config::load(&fs).unwrap();
    let session = GameSession::load(fs, &config, false).unwrap();
    session.lua().load("scene.switch('play')").exec().unwrap();
    session.tick(1.0 / 60.0).unwrap();
    let current = session
        .lua()
        .globals()
        .get::<mlua::Table>("scene")
        .unwrap()
        .get::<mlua::Function>("current")
        .unwrap()
        .call::<String>(())
        .unwrap();
    assert_eq!(current, "play");
    let shape_count = session
        .state()
        .borrow()
        .frame
        .commands
        .iter()
        .filter(|command| matches!(command, DrawCommand::AffineQuad { .. }))
        .count();
    assert_eq!(
        shape_count, 3,
        "native player sprite and two prefab markers should draw"
    );
    assert_eq!(session.state().borrow().physics.body_count(), 3);
    assert_eq!(session.state().borrow().assets.texture_count(), 1);
}

#[test]
fn serialized_scene_loads_script_from_project_and_rejects_parent_path() {
    use kairo_core::scene_graph::{NodeKind, SceneGraph};
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("scenes")).unwrap();
    std::fs::create_dir(root.path().join("scripts")).unwrap();
    std::fs::write(
        root.path().join("scripts/check.lua"),
        "return {ready = function(self) self.node:setProperty('ready', true) end}",
    )
    .unwrap();
    let mut graph = SceneGraph::new("Loaded").unwrap();
    let node = graph
        .create(graph.root(), NodeKind::ScriptNode, "Check")
        .unwrap();
    graph
        .set_script(node, Some("scripts/check.lua".into()))
        .unwrap();
    std::fs::write(
        root.path().join("scenes/loaded.scene"),
        graph.to_json().unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.path().join("main.lua"),
        r#"
        assert(not pcall(scene.load, '../outside.scene'))
        local loaded = scene.load('scenes/loaded.scene')
        local check = loaded.graph:findPath('Loaded/Check')
        scene.switch(loaded)
        function game.update() assert(check:getProperty('ready') == true) end
    "#,
    )
    .unwrap();
    let session = GameSession::load(
        ProjectFs::new(root.path()).unwrap(),
        &Config::default(),
        false,
    )
    .unwrap();
    session.tick(0.016).unwrap();
}

#[test]
fn hierarchical_prefabs_expand_nesting_override_properties_and_reject_cycles() {
    use kairo_core::scene_graph::{NodeKind, SceneGraph};
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("prefabs")).unwrap();
    let mut enemy = SceneGraph::new("Enemy").unwrap();
    enemy
        .create(enemy.root(), NodeKind::Sprite, "Visual")
        .unwrap();
    enemy
        .set_property(enemy.root(), "hp", serde_json::json!(10))
        .unwrap();
    std::fs::write(
        root.path().join("prefabs/enemy.prefab"),
        enemy.to_json().unwrap(),
    )
    .unwrap();
    let mut strong = SceneGraph::new("Strong").unwrap();
    let nested = strong
        .create(strong.root(), NodeKind::Node2D, "EnemyInstance")
        .unwrap();
    strong
        .set_prefab(nested, Some("prefabs/enemy.prefab".into()))
        .unwrap();
    std::fs::write(
        root.path().join("prefabs/strong.prefab"),
        strong.to_json().unwrap(),
    )
    .unwrap();
    let mut a = SceneGraph::new("A").unwrap();
    let a_ref = a.create(a.root(), NodeKind::Node2D, "BRef").unwrap();
    a.set_prefab(a_ref, Some("prefabs/b.prefab".into()))
        .unwrap();
    std::fs::write(root.path().join("prefabs/a.prefab"), a.to_json().unwrap()).unwrap();
    let mut b = SceneGraph::new("B").unwrap();
    let b_ref = b.create(b.root(), NodeKind::Node2D, "ARef").unwrap();
    b.set_prefab(b_ref, Some("prefabs/a.prefab".into()))
        .unwrap();
    std::fs::write(root.path().join("prefabs/b.prefab"), b.to_json().unwrap()).unwrap();
    std::fs::write(root.path().join("main.lua"), r#"
        local level = scene.new('Level')
        local first = prefab.instantiate('enemy', level.root, {hp=50})
        local second = prefab.instantiate('enemy', level.root)
        assert(first.id ~= second.id and first:getProperty('hp') == 50 and second:getProperty('hp') == 10)
        local strong = prefab.instantiate('strong', level.root)
        assert(strong:findChild('EnemyInstance'):findChild('Visual') ~= nil)
        local ok, message = pcall(prefab.instantiate, 'a', level.root)
        assert(not ok and tostring(message):find('recursive prefab dependency'))
    "#).unwrap();
    GameSession::load(
        ProjectFs::new(root.path()).unwrap(),
        &Config::default(),
        false,
    )
    .unwrap();
}

#[test]
fn animation_frames_become_texture_regions_without_exposing_internals() {
    let (_root, session) = project(
        r#"
        local texture = graphics.loadTexture('pixel.png')
        clip = animation.new(texture, {
            {x=0,y=0,w=1,h=2,duration=0.25}, {x=1,y=0,w=1,h=2,duration=0.25}
        })
        clip:update(0.25)
        assert(clip:getFrame()==2)
        clip:setLooping(false)
        clip:update(10)
        assert(clip:isFinished())
        function game.draw() graphics.draw(clip, 12, 14) end
    "#,
    );
    session.tick(1.0 / 60.0).unwrap();
    let state = session.state().borrow();
    let DrawCommand::Quad(quad) = &state.frame.commands[0] else {
        panic!("expected sprite quad")
    };
    assert_eq!(quad.uv_min.to_array(), [0.5, 0.0]);
    assert_eq!(quad.uv_max.to_array(), [1.0, 1.0]);
    assert_eq!(quad.size.to_array(), [1.0, 2.0]);
}

#[test]
fn retained_ui_handles_press_release_between_frames() {
    let (_root, session) = project(
        r#"
        clicks = 0
        function game.load()
            local panel = ui.panel({x=10,y=10,width=200,height=100})
            panel:add(ui.button('Click', function() clicks=clicks+1 end))
        end
    "#,
    );
    session.ui_pointer(true, 30.0, 30.0, 1).unwrap();
    session.ui_pointer(false, 30.0, 30.0, 1).unwrap();
    session.tick(1.0 / 60.0).unwrap();
    assert_eq!(session.lua().globals().get::<u32>("clicks").unwrap(), 1);
    assert!(session.state().borrow().frame.commands.len() > 1);
}

#[test]
fn debugui_is_phase_scoped_and_collects_native_widgets() {
    let (_root, session) = project(
        r#"
        assert(not pcall(debugui.button, 'Outside'))
        function game.debugUI()
            debugui.window('Controls', function()
                debugui.text('Native panel')
                assert(debugui.slider('Speed',100,0,50)==50)
                assert(debugui.checkbox('Enabled',true))
                assert(not debugui.button('Reset'))
            end)
        end
    "#,
    );
    session.tick(1.0 / 60.0).unwrap();
    let state = session.state().borrow();
    assert_eq!(state.debug_ui.windows.len(), 1);
    assert_eq!(state.debug_ui.windows[0].widgets.len(), 4);
}

#[test]
fn replay_restores_lua_rng_and_body_state_at_frame_boundary() {
    let (_root, session) = project(
        r#"
        state = {x=8,nested={value=12}}
        nested = state.nested
        body = physics.newRectangle('dynamic',10,20,8,8)
        replay.register('state',state)
        random.seed(123)
        replay.capture()
        expected = random.integer(0,1000000)
        function game.update(dt) state.x=state.x+1 end
    "#,
    );
    for _ in 0..5 {
        session.tick(1.0 / 60.0).unwrap();
    }
    session
        .lua()
        .load("state.nested.value=99; body:setPosition(100,200); replay.seek(1)")
        .exec()
        .unwrap();
    session.tick(1.0 / 60.0).unwrap();
    session
        .lua()
        .load(
            r#"
        assert(state.x==8 and nested==state.nested and nested.value==12)
        local x,y=body:getPosition()
        assert(math.abs(x-10)<0.001 and math.abs(y-20)<0.001)
        assert(replay.stats().paused)
        assert(random.integer(0,1000000)==expected)
        replay.resume()
    "#,
        )
        .exec()
        .unwrap();
    session.tick(1.0 / 60.0).unwrap();
    session
        .lua()
        .load("assert(state.x==9 and not replay.stats().paused)")
        .exec()
        .unwrap();
}

#[test]
fn replay_rejects_changed_physics_topology_before_lua_restoration() {
    let (_root, session) = project(
        r#"
        state={value=1}; replay.register('state',state); replay.capture()
        state.value=2
        body=physics.newCircle('dynamic',0,0,4)
        replay.seek(1)
    "#,
    );
    assert!(session.tick(0.016).is_err());
    session
        .lua()
        .load("assert(state.value==2); assert(body:getPosition()==0)")
        .exec()
        .unwrap();
}

#[test]
fn reload_hooks_transfer_plain_state_and_failed_replacements_keep_old_vm() {
    let (root, mut session) = project(
        r#"
        state={score=7}
        function game.saveState() return state end
        function game.restoreState(saved) state=saved end
    "#,
    );
    std::fs::write(
        root.path().join("main.lua"),
        r#"
        state={score=0}; version=2
        function game.saveState() return state end
        function game.restoreState(saved) state=saved end
    "#,
    )
    .unwrap();
    session.reload(&Config::default(), false).unwrap();
    session
        .lua()
        .load("assert(state.score==7 and version==2)")
        .exec()
        .unwrap();
    std::fs::write(
        root.path().join("main.lua"),
        "function game.load() error('reject candidate') end",
    )
    .unwrap();
    assert!(session.reload(&Config::default(), false).is_err());
    session
        .lua()
        .load("assert(state.score==7 and version==2)")
        .exec()
        .unwrap();
}

#[test]
fn candidate_load_cannot_write_saves() {
    let (_root, session) = project(
        r#"
        local ok,err=pcall(save.write,'settings.json',{value=1})
        assert(not ok and tostring(err):find('during game.load'))
        assert(not pcall(save.remove,'settings.json'))
        assert(not pcall(graphics.loadFont,'pixel.png',24))
    "#,
    );
    session.tick(0.016).unwrap();
}

#[test]
fn actual_animation_tilemap_micro_ui_replay_and_link_examples_tick_headlessly() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    for name in [
        "animation",
        "tilemap",
        "micro",
        "ui",
        "replay",
        "link",
        "gamepad",
        "fonts",
    ] {
        let fs = ProjectFs::new(root.join(name)).unwrap();
        let config = Config::load(&fs).unwrap();
        let session = GameSession::load(fs, &config, false).unwrap();
        for _ in 0..5 {
            session.tick(1.0 / 60.0).unwrap();
        }
        assert!(
            !session.state().borrow().frame.commands.is_empty(),
            "{name}"
        );
    }
}

#[test]
fn relay_dusk_starts_fights_and_pauses_headlessly() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/relay-dusk");
    let fs = ProjectFs::new(root).unwrap();
    let config = Config::load(&fs).unwrap();
    let session = GameSession::load(fs, &config, false).unwrap();
    session.state().borrow_mut().input.set_key("enter", true);
    session.tick(1.0 / 60.0).unwrap();
    session.state().borrow_mut().input.set_key("enter", false);
    session
        .lua()
        .load("assert(scene.current() == 'mission')")
        .exec()
        .unwrap();
    {
        let mut state = session.state().borrow_mut();
        state.input.set_key("d", true);
        state.input.set_key("space", true);
        state.input.mouse_x = 1050.0;
        state.input.mouse_y = 355.0;
    }
    for _ in 0..180 {
        session.tick(1.0 / 60.0).unwrap();
    }
    assert!(!session.state().borrow().frame.commands.is_empty());
    {
        let mut state = session.state().borrow_mut();
        state.input.set_key("space", false);
        state.input.set_key("d", false);
        state.input.set_key("escape", true);
    }
    session.tick(1.0 / 60.0).unwrap();
    session
        .lua()
        .load("assert(scene.current() == 'pause')")
        .exec()
        .unwrap();
}

#[test]
fn focus_cancellation_does_not_synthesize_a_ui_click() {
    let (_root, session) = project(
        r#"
        clicks=0
        local panel=ui.panel({x=0,y=0,width=100,height=100})
        panel:add(ui.button('Click',function() clicks=clicks+1 end))
    "#,
    );
    session.ui_pointer(true, 20.0, 20.0, 1).unwrap();
    session.tick(0.016).unwrap();
    session.ui_cancel_pointer().unwrap();
    session.ui_pointer(false, 20.0, 20.0, 1).unwrap();
    session.tick(0.016).unwrap();
    session.lua().load("assert(clicks==0)").exec().unwrap();
}
