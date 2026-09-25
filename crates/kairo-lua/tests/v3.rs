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
