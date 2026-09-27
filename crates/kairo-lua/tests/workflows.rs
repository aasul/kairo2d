use kairo_core::inspector::InspectUpdate;
use kairo_core::profiler::DebugCommand;
use kairo_core::{Config, ProjectFs};
use kairo_lua::GameSession;

fn session(source: &str) -> (tempfile::TempDir, GameSession) {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("main.lua"), source).unwrap();
    let session = GameSession::load(
        ProjectFs::new(root.path()).unwrap(),
        &Config::default(),
        false,
    )
    .unwrap();
    (root, session)
}

#[test]
fn scene_stack_actions_and_overlay_callbacks_share_one_lifecycle() {
    let (_root, session) = session(
        r#"
        local play, pause = scene.new('play'), scene.new('pause')
        local updates = 0
        pause.pause_physics = true
        input.bind('pause', {keyboard={'escape'}})
        input.bindAxis('move', {negative={'a'}, positive={'d'}})
        function play:update(dt)
            updates = updates + 1
            assert(input.axis('move') == 1)
            if input.pressed('pause') then scene.push('pause') end
        end
        function pause:update() assert(updates == 1) end
        function play:draw() graphics.rectangle('fill', 0, 0, 12, 12) end
        function pause:draw() graphics.rectangle('fill', 10, 10, 8, 8) end
        scene.register(play); scene.register(pause)
        function game.load() scene.switch('play') end
    "#,
    );
    session.state().borrow_mut().input.set_key("d", true);
    session.state().borrow_mut().input.set_key("escape", true);
    session.tick(0.016).unwrap();
    assert_eq!(
        session.telemetry().inspection.scenes.stack,
        ["play", "pause"]
    );
    assert_eq!(session.state().borrow().frame.commands.len(), 2);
    session.tick(0.016).unwrap();
}

#[test]
fn inspector_edits_are_opt_in_session_scoped_and_compare_expected_value() {
    let (_root, session) = session(
        r#"
        player = {speed=200, health=3, position={10,20}}
        inspector.expose('player.speed', player, 'speed', {writable=true,min=0,max=500})
        inspector.expose('player.health', player, 'health')
        inspector.expose('player.position', player, 'position', {kind='vector',writable=true})
    "#,
    );
    let snapshot = session.telemetry().inspection;
    assert!(snapshot.error.is_none(), "{:?}", snapshot.error);
    let speed = snapshot.node("player.speed").unwrap();
    let update = InspectUpdate {
        session: snapshot.session,
        path: speed.path.clone(),
        expected: speed.value.clone(),
        value: 300.into(),
    };
    session
        .debug_command(DebugCommand::Inspect {
            update: Box::new(update.clone()),
        })
        .unwrap();
    assert!(session
        .debug_command(DebugCommand::Inspect {
            update: Box::new(update)
        })
        .is_err());
    let health = snapshot.node("player.health").unwrap();
    assert!(session
        .debug_command(DebugCommand::Inspect {
            update: Box::new(InspectUpdate {
                session: snapshot.session,
                path: health.path.clone(),
                expected: health.value.clone(),
                value: 4.into(),
            })
        })
        .is_err());
    session
        .lua()
        .load("assert(player.speed == 300 and player.health == 3)")
        .exec()
        .unwrap();
    let position = snapshot.node("player.position").unwrap();
    session
        .debug_command(DebugCommand::Inspect {
            update: Box::new(InspectUpdate {
                session: snapshot.session,
                path: position.path.clone(),
                expected: position.value.clone(),
                value: serde_json::json!([30, 40]),
            }),
        })
        .unwrap();
    session
        .lua()
        .load("assert(player.position[1] == 30 and player.position[2] == 40)")
        .exec()
        .unwrap();
}

#[test]
fn particle_simulation_is_native_bounded_and_draws_into_the_frame_queue() {
    let (_root, session) = session(
        r#"
        sparks = particles.new({rate=0,max_particles=4,lifetime={0.1,0.1},speed={0,0}})
        assert(sparks:emit(900) == 4)
        assert(not pcall(particles.new,{lifetime={2,1}}))
        function game.update(dt) sparks:update(dt) end
        function game.draw() sparks:draw() end
    "#,
    );
    session.tick(0.01).unwrap();
    assert_eq!(session.state().borrow().frame.commands.len(), 4);
    assert_eq!(session.telemetry().profile.active_particles, 4);
    assert_eq!(session.telemetry().profile.active_emitters, 1);
    session.tick(0.1).unwrap();
    assert!(session.state().borrow().frame.commands.is_empty());
    assert_eq!(session.telemetry().profile.active_particles, 0);
}

#[test]
fn bookmarks_restore_registered_values_and_controlled_rng_after_eviction() {
    let (_root, session) = session(
        r#"
        data={x=5}; replay.register('data',data); random.seed(17)
        bookmark = replay.bookmark('before move',''); expected = random.float()
        data.x=900; random.float()
    "#,
    );
    let sample = session.telemetry();
    let id = session.lua().globals().get::<u64>("bookmark").unwrap();
    session.state().borrow_mut().replay.timeline.clear();
    session
        .debug_command(DebugCommand::BookmarkJump {
            session: sample.inspection.session,
            id,
        })
        .unwrap();
    session.tick(0.016).unwrap();
    session
        .lua()
        .load("assert(data.x == 5); assert(random.float() == expected)")
        .exec()
        .unwrap();
    assert!(session.telemetry().replay.paused);
    assert_eq!(session.telemetry().bookmarks.len(), 1);
}

#[test]
fn release_profile_rejects_runtime_controls_without_disabling_games() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("main.lua"),
        "function game.draw() graphics.rectangle('fill',0,0,1,1) end",
    )
    .unwrap();
    let mut config = Config::default();
    config
        .apply_profile(kairo_core::profile::BuildProfile::Release)
        .unwrap();
    let session = GameSession::load(ProjectFs::new(root.path()).unwrap(), &config, false).unwrap();
    assert!(!session.telemetry().tools_enabled);
    assert!(session.debug_command(DebugCommand::Pause).is_err());
    session.tick(0.016).unwrap();
    assert_eq!(session.state().borrow().frame.commands.len(), 1);
}

#[test]
fn mixer_validation_and_muting_work_without_an_audio_device() {
    let (_root, session) = session(
        r#"
        audio.setBusVolume('music',0.3,0.2); audio.muteBus('music',true)
        assert(not pcall(audio.setBusVolume,'missing',1))
        assert(not pcall(audio.setBusVolume,'sfx',2))
    "#,
    );
    let music = session
        .telemetry()
        .mixer
        .into_iter()
        .find(|bus| bus.name == "music")
        .unwrap();
    assert_eq!(music.volume, 0.3);
    assert!(music.muted);
}
