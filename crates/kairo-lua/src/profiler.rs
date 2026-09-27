use crate::state::{with_state, SharedState};
use kairo_core::{Camera, Color, DrawCommand, Quad, Transform};
use mlua::{Lua, LuaSerdeExt};
use std::time::Instant;

pub(crate) fn register(lua: &Lua, state: &SharedState) -> mlua::Result<()> {
    let module = lua.create_table()?;
    let s = state.clone();
    module.set(
        "show",
        lua.create_function(move |_, enabled: bool| {
            with_state(&s, |state| {
                state.show_profiler = enabled && state.development_tools;
                Ok(())
            })
        })?,
    )?;
    let s = state.clone();
    module.set(
        "stats",
        lua.create_function(move |lua, ()| {
            let mut sample = with_state(&s, |state| Ok(state.profile.for_telemetry()))?;
            let scene: mlua::Table = lua.globals().get("scene")?;
            let info: mlua::Table = scene.get::<mlua::Function>("info")?.call(())?;
            sample.active_nodes = info.get("active_nodes")?;
            lua.to_value(&sample)
        })?,
    )?;
    if state.borrow().profile.profiling_enabled {
        let s = state.clone();
        module.set(
            "_invoke",
            lua.create_function(
                move |_,
                      (script, scene, node_path, callback, function, arguments): (
                    String,
                    String,
                    String,
                    String,
                    mlua::Function,
                    mlua::MultiValue,
                )| {
                    let start = Instant::now();
                    let result = function.call::<()>(arguments);
                    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
                    if let Ok(mut state) = s.try_borrow_mut() {
                        state
                            .profile
                            .record_callback(&script, &scene, &node_path, &callback, elapsed_ms);
                    }
                    result
                },
            )?,
        )?;
    }
    let debug = lua.create_table()?;
    debug.set("showProfiler", module.get::<mlua::Function>("show")?)?;
    for (name, target) in [
        ("drawPhysics", 0),
        ("drawBounds", 1),
        ("drawVelocities", 2),
        ("drawOrigins", 3),
        ("drawNodeNames", 4),
        ("drawCamera", 5),
    ] {
        let s = state.clone();
        debug.set(
            name,
            lua.create_function(move |_, enabled: bool| {
                with_state(&s, |state| {
                    let enabled = enabled && state.development_tools;
                    match target {
                        0 => state.debug_draw.physics = enabled,
                        1 => state.debug_draw.bounds = enabled,
                        2 => state.debug_draw.velocities = enabled,
                        3 => state.debug_draw.origins = enabled,
                        4 => state.debug_draw.names = enabled,
                        _ => state.debug_draw.camera = enabled,
                    }
                    Ok(())
                })
            })?,
        )?;
    }
    lua.globals().set("debug", debug)?;
    lua.globals().set("profiler", module)
}

pub(crate) fn overlay(state: &mut crate::state::EngineState) -> anyhow::Result<()> {
    if !state.show_profiler {
        return Ok(());
    }
    let profile = &state.profile;
    let text = format!("FPS {:5.1}  frame {:5.2} ms\nLua U {:5.2} D {:5.2} physics {:5.2}\ncommands {} batches {} vertices {}\ntextures {}  {:.1} MiB  reloads {}",
        state.fps, profile.frame_ms, profile.update_ms, profile.draw_ms, profile.physics_ms,
        profile.commands, profile.batches, profile.vertices, profile.textures,
        profile.texture_bytes as f64 / (1024.0 * 1024.0), profile.reloads);
    state.frame.push(DrawCommand::Viewport(None))?;
    state.frame.push(DrawCommand::Scissor(None))?;
    state.frame.push(DrawCommand::Quad(Quad {
        texture: None,
        size: [state.width.min(470) as f32, 52.0].into(),
        uv_min: [0.0, 0.0].into(),
        uv_max: [1.0, 1.0].into(),
        transform: Transform::default(),
        camera: Camera::default(),
        color: Color([0.025, 0.03, 0.045, 0.95]),
    }))?;
    state.frame.push(DrawCommand::Text {
        text,
        position: [6.0, 6.0].into(),
        scale: 1.0,
        camera: Camera::default(),
        color: Color::WHITE,
    })
}
