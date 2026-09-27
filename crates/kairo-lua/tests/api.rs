use kairo_core::{Config, DrawCommand, ProjectFs};
use kairo_lua::{check_project, GameSession};
use mlua::Table;

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
fn modules_are_registered_without_unsafe_standard_libraries() {
    let (_root, session) = project("");
    for name in [
        "game",
        "graphics",
        "audio",
        "keyboard",
        "mouse",
        "window",
        "timer",
        "filesystem",
        "assets",
        "physics",
    ] {
        assert!(session.lua().globals().get::<Table>(name).is_ok(), "{name}");
    }
    session
        .lua()
        .load(
            r#"
        assert(os == nil and io == nil)
        assert(type(debug) == "table" and debug.getregistry == nil and debug.getupvalue == nil)
        assert(dofile == nil and loadfile == nil and package.loadlib == nil)
        assert(type(graphics.rectangle) == 'function')
        assert(type(physics.newRectangle) == 'function')
        assert(audio.isAvailable() == false)
    "#,
        )
        .exec()
        .unwrap();
}

#[test]
fn lifecycle_orders_load_update_draw_and_clamps_long_frames() {
    let (_root, session) = project(
        r#"
        events = ''
        function game.load() events = events .. 'L' end
        function game.update(dt)
            assert(dt <= 0.101)
            assert(timer.getDelta() == dt)
            events = events .. 'U'
        end
        function game.draw()
            events = events .. 'D'
            graphics.rectangle('fill', 0, 0, 10, 10)
        end
    "#,
    );
    session.tick(4.0).unwrap();
    session.tick(0.05).unwrap();
    assert_eq!(
        session.lua().globals().get::<String>("events").unwrap(),
        "LUDUD"
    );
    let state = session.state().borrow();
    assert!((state.elapsed - 0.15).abs() < 0.00001);
    assert_eq!(state.frame.commands.len(), 1);
}

#[test]
fn texture_handles_are_cached_and_sprite_options_reach_the_queue() {
    let (_root, session) = project(
        r#"
        local a = graphics.loadTexture('pixel.png')
        local b = graphics.loadTexture('./pixel.png')
        local w, h = a:getDimensions()
        assert(w == 2 and h == 2)
        assert(assets.stats().textures == 1)
        function game.draw()
            graphics.setColor(0.5, 0.6, 0.7, 0.8)
            graphics.draw(b, {x=12, y=34, rotation=0.5, scale_x=2, scale_y=3,
                origin_x=1, origin_y=1, source={x=0, y=0, width=1, height=2}})
        end
    "#,
    );
    session.tick(0.016).unwrap();
    let state = session.state().borrow();
    let DrawCommand::Quad(quad) = &state.frame.commands[0] else {
        panic!("expected a sprite")
    };
    assert!(quad.texture.is_some());
    assert_eq!(quad.transform.position.to_array(), [12.0, 34.0]);
    assert_eq!(quad.transform.scale.to_array(), [2.0, 3.0]);
    assert_eq!(quad.uv_max.to_array(), [0.5, 1.0]);
    assert_eq!(quad.color.0, [0.5, 0.6, 0.7, 0.8]);
}

#[test]
fn invalid_arguments_are_lua_errors_not_rust_panics() {
    let (_root, session) = project(
        r#"
        assert(not pcall(graphics.rectangle, 'fill', 0, 0, 1, 1))
        assert(not pcall(graphics.setColor, 255, 0, 0))
        assert(not pcall(graphics.setCamera, 0, 0, 0))
        assert(not pcall(mouse.isDown, 0))
        assert(not pcall(window.setSize, 0, 0))
        local image = graphics.loadTexture('pixel.png')
        function game.draw()
            assert(not pcall(graphics.draw, 123, 0, 0))
            assert(not pcall(graphics.draw, image, {scale=2}))
            assert(not pcall(graphics.draw, image, {source={x=1, y=0, width=2, height=2}}))
            assert(not pcall(graphics.rectangle, 'unknown', 0, 0, 1, 1))
        end
    "#,
    );
    session.tick(0.016).unwrap();
}

#[test]
fn clear_discards_previous_commands_and_failed_draw_resets_the_phase() {
    let (_root, session) = project(
        r#"
        function game.draw()
            graphics.rectangle('fill', 0, 0, 10, 10)
            graphics.clear(0, 0, 0)
            graphics.rectangle('line', 0, 0, 20, 20, 2)
        end
    "#,
    );
    session.tick(0.016).unwrap();
    assert_eq!(session.state().borrow().frame.commands.len(), 4);
    session
        .lua()
        .load("function game.draw() error('draw failed') end")
        .exec()
        .unwrap();
    assert!(session.tick(0.016).is_err());
    assert!(session.state().borrow().frame.commands.is_empty());
    assert!(session
        .lua()
        .load("graphics.rectangle('fill', 0, 0, 10, 10)")
        .exec()
        .is_err());
}

#[test]
fn input_callbacks_observe_updated_polling_state() {
    let (_root, session) = project(
        r#"
        pressed = 0
        function game.keyPressed(key)
            assert(keyboard.isDown(key))
            pressed = pressed + 1
        end
        function game.mousePressed(x, y, button)
            assert(mouse.isDown(button))
            assert(x == 20 and y == 30)
        end
    "#,
    );
    session.state().borrow_mut().input.set_key("space", true);
    session.call("keyPressed", "space").unwrap();
    session.state().borrow_mut().input.set_button(1, true);
    session.call("mousePressed", (20, 30, 1)).unwrap();
    assert_eq!(session.lua().globals().get::<i32>("pressed").unwrap(), 1);
}

#[test]
fn project_modules_are_cached_and_cannot_escape_the_root() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("player")).unwrap();
    std::fs::write(
        root.path().join("player/init.lua"),
        "loads = (loads or 0) + 1; return {speed=200}",
    )
    .unwrap();
    std::fs::write(
        root.path().join("main.lua"),
        r#"
        local player = require('player')
        assert(player.speed == 200)
        assert(require('player') == player and loads == 1)
        assert(not pcall(require, '../outside'))
        assert(not pcall(filesystem.read, '../outside'))
        assert(not pcall(filesystem.exists, '/etc/passwd'))
    "#,
    )
    .unwrap();
    GameSession::load(
        ProjectFs::new(root.path()).unwrap(),
        &Config::default(),
        false,
    )
    .unwrap();
}

#[test]
fn syntax_errors_and_nested_module_errors_keep_their_filenames() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("main.lua"),
        "function game.load()\n local x = )\nend",
    )
    .unwrap();
    let result = GameSession::load(
        ProjectFs::new(root.path()).unwrap(),
        &Config::default(),
        false,
    );
    let error = match result {
        Ok(_) => panic!("invalid Lua accepted"),
        Err(error) => format!("{error:#}"),
    };
    assert!(error.contains("main.lua:2"), "{error}");
    std::fs::write(root.path().join("main.lua"), "require('broken')").unwrap();
    std::fs::write(root.path().join("broken.lua"), "error('module exploded')").unwrap();
    let result = GameSession::load(
        ProjectFs::new(root.path()).unwrap(),
        &Config::default(),
        false,
    );
    let error = match result {
        Ok(_) => panic!("module error ignored"),
        Err(error) => format!("{error:#}"),
    };
    assert!(error.contains("broken.lua:1"), "{error}");
    assert!(error.contains("module exploded"));
}

#[test]
fn physics_bodies_use_safe_methods_and_destroyed_bodies_fail() {
    let (_root, session) = project(
        r#"
        body = physics.newRectangle('dynamic', 100, 0, 20, 20)
        body:setVelocity(10, 0)
        function game.draw()
            local x, y = body:getPosition()
            graphics.rectangle('fill', x - 10, y - 10, 20, 20)
        end
    "#,
    );
    for _ in 0..60 {
        session.tick(1.0 / 60.0).unwrap();
    }
    session
        .lua()
        .load(
            r#"
        local x, y = body:getPosition()
        assert(x > 109 and y > 470)
        body:destroy()
        local ok, message = pcall(function() return body:getPosition() end)
        assert(not ok and tostring(message):find('destroyed'))
        assert(assets.stats().bodies == 0)
    "#,
        )
        .exec()
        .unwrap();
}

#[test]
fn reload_is_a_fresh_vm_with_fresh_resource_handles() {
    let (root, first) = project(
        r#"
        counter = (counter or 0) + 1
        local image = graphics.loadTexture('pixel.png')
        function game.draw() graphics.draw(image, 0, 0) end
    "#,
    );
    first.tick(0.016).unwrap();
    let second = GameSession::load(
        ProjectFs::new(root.path()).unwrap(),
        &Config::default(),
        false,
    )
    .unwrap();
    second.tick(0.016).unwrap();
    assert_eq!(second.lua().globals().get::<i32>("counter").unwrap(), 1);
    let texture = |session: &GameSession| {
        let state = session.state().borrow();
        match &state.frame.commands[0] {
            DrawCommand::Quad(q) => q.texture,
            _ => None,
        }
    };
    assert_ne!(texture(&first), texture(&second));
}

#[test]
fn check_compiles_nested_sources_without_running_them() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("main.lua"), "error('must not run')").unwrap();
    std::fs::create_dir(root.path().join("code")).unwrap();
    std::fs::write(root.path().join("code/util.lua"), "return 42").unwrap();
    let fs = ProjectFs::new(root.path()).unwrap();
    assert_eq!(check_project(&fs).unwrap(), 2);
    std::fs::write(root.path().join("code/util.lua"), "return )").unwrap();
    assert!(format!("{:#}", check_project(&fs).unwrap_err()).contains("code/util.lua"));
}

#[test]
fn failed_reload_retains_running_state_and_success_resets_modules() {
    let (root, mut session) = project("counter=5; function game.update() counter=counter+1 end");
    session.state().borrow_mut().input.set_key("d", true);
    std::fs::write(
        root.path().join("main.lua"),
        "function game.load() error('reload failed') end",
    )
    .unwrap();
    assert!(session.reload(&Config::default(), false).is_err());
    session.tick(0.016).unwrap();
    assert_eq!(session.lua().globals().get::<i32>("counter").unwrap(), 6);
    std::fs::write(root.path().join("main.lua"), "counter=require('counter')").unwrap();
    std::fs::write(root.path().join("counter.lua"), "return 41").unwrap();
    session.reload(&Config::default(), false).unwrap();
    assert_eq!(session.lua().globals().get::<i32>("counter").unwrap(), 41);
    assert!(session.state().borrow().input.key_down("d"));
    std::fs::write(root.path().join("counter.lua"), "return 42").unwrap();
    session.reload(&Config::default(), false).unwrap();
    assert_eq!(session.lua().globals().get::<i32>("counter").unwrap(), 42);
}

#[test]
fn viewport_clear_and_scissor_state_reach_the_render_queue() {
    let (_root, session) = project(
        r#"
        function game.draw()
            graphics.setViewport(10, 20, 100, 80)
            graphics.setScissor(10, 20, 40, 40)
            graphics.clear(0, 0, 0)
            local x,y=graphics.screenToWorld(10,20)
            assert(x==0 and y==0)
            graphics.rectangle('fill', 0,0,100,80)
            graphics.resetScissor()
            graphics.resetViewport()
            assert(not pcall(graphics.setViewport, 0,0,999999,10))
        end
    "#,
    );
    session.tick(0.016).unwrap();
    let state = session.state().borrow();
    assert_eq!(state.frame.commands.len(), 5);
    assert!(matches!(
        state.frame.commands[0],
        DrawCommand::Viewport(Some(_))
    ));
    assert!(matches!(
        state.frame.commands[1],
        DrawCommand::Scissor(Some(_))
    ));
}

#[test]
fn released_texture_userdata_cannot_draw_a_new_resource() {
    let (_root, session) = project(
        r#"
        local old=graphics.loadTexture('pixel.png')
        graphics.setFilter(old,'linear')
        graphics.releaseTexture(old)
        local current=graphics.loadTexture('pixel.png')
        assert(not pcall(graphics.setFilter,old,'nearest'))
        function game.draw()
            assert(not pcall(graphics.draw,old,0,0))
            assert(not pcall(graphics.releaseTexture,current))
            graphics.draw(current,0,0)
        end
    "#,
    );
    session.tick(0.016).unwrap();
    assert_eq!(session.state().borrow().assets.texture_count(), 1);
}
