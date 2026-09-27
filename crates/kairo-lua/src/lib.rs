//! Runs a game in its own Lua VM and resource session. Reloading creates a new session.
mod actions;
mod animation;
mod audio;
mod debugui;
mod fonts;
mod gamepad;
mod graphics;
mod inspector;
mod modules;
mod particles;
mod physics;
mod profiler;
pub mod replay;
mod save;
mod scene_audio;
mod scene_graph;
mod scene_physics;
mod scene_render;
mod services;
mod state;
mod tilemap;
mod ui;

pub use state::{EngineState, SharedState, WindowCommand};

use anyhow::{bail, ensure, Context, Result};
use kairo_core::{Camera, Color, Config, ProjectFs};
use mlua::{
    ChunkMode, HookTriggers, IntoLuaMulti, Lua, LuaOptions, LuaSerdeExt, StdLib, Table, Value,
    VmState,
};
use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;
use std::time::{Duration, Instant};

pub struct GameSession {
    lua: Lua,
    state: SharedState,
    deadline: Rc<Cell<Instant>>,
    extensions: Rc<RefCell<Vec<Box<dyn kairo_core::extensions::NativeExtension>>>>,
}

impl GameSession {
    pub fn load(fs: ProjectFs, config: &Config, audio_enabled: bool) -> Result<Self> {
        let lua = create_lua()?;
        let state = Rc::new(RefCell::new(EngineState::new(
            fs.clone(),
            config,
            audio_enabled,
        )?));
        graphics::register(&lua, &state)?;
        fonts::register(&lua, &state)?;
        animation::register(&lua, &state)?;
        audio::register(&lua, &state)?;
        physics::register(&lua, &state)?;
        services::register(&lua, &state)?;
        gamepad::register(&lua, &state)?;
        actions::register(&lua, &state)?;
        inspector::register(&lua, &state)?;
        particles::register(&lua, &state)?;
        save::register(&lua, &state)?;
        tilemap::register(&lua, &state)?;
        replay::register(&lua, &state)?;
        profiler::register(&lua, &state)?;
        debugui::register(&lua, &state)?;
        ui::register(&lua)?;
        scene_graph::register(&lua, fs.clone(), &state)?;
        modules::install(&lua, fs.clone())?;
        lua.globals().set("game", lua.create_table()?)?;
        for (name, source) in [
            ("Events", include_str!("builtin/events.lua")),
            ("scene", include_str!("builtin/scene.lua")),
            ("animator", include_str!("builtin/animator.lua")),
            ("prefab", include_str!("builtin/prefab.lua")),
            ("localization", include_str!("builtin/localization.lua")),
        ] {
            let module: Table = lua
                .load(source)
                .set_name(format!("@kairo/{name}.lua"))
                .eval()?;
            lua.globals().set(name, module)?;
        }
        let deadline = Rc::new(Cell::new(Instant::now() + Duration::from_secs(2)));
        let hook_deadline = deadline.clone();
        lua.set_hook(
            HookTriggers {
                every_nth_instruction: Some(10_000),
                ..Default::default()
            },
            move |_, _| {
                if Instant::now() > hook_deadline.get() {
                    return Err(mlua::Error::RuntimeError(
                        "Lua callback exceeded the two-second instruction watchdog".into(),
                    ));
                }
                Ok(VmState::Continue)
            },
        );
        let session = Self {
            lua,
            state,
            deadline,
            extensions: Rc::new(RefCell::new(Vec::new())),
        };
        let source = fs
            .read_text("main.lua")
            .context("cannot load game entry point")?;
        session.arm_watchdog();
        session
            .lua
            .load(&source)
            .set_name("@main.lua")
            .set_mode(ChunkMode::Text)
            .exec()
            .context("Kairo2D Lua Error while loading main.lua")?;
        session.call("load", ())?;
        session.state.borrow_mut().loading = false;
        log::info!("Loaded main.lua");
        Ok(session)
    }

    /// Prepare a separate VM. Explicit saveState/restoreState hooks can retain plain Lua data.
    pub fn replacement(&self, fs: ProjectFs, config: &Config, audio_enabled: bool) -> Result<Self> {
        let state = self
            .state
            .try_borrow()
            .context("engine state is already in use")?;
        let input = state.input.clone();
        let reloads = state.profile.reloads.saturating_add(1);
        drop(state);
        self.arm_watchdog();
        let game: Table = self.lua.globals().get("game")?;
        let saved = match game.get::<Value>("saveState")? {
            Value::Nil => None,
            Value::Function(function) => {
                let value: Value = function.call(())?;
                let replay: Table = self.lua.globals().get("replay")?;
                let encoded: Value = replay.get::<mlua::Function>("_encode")?.call(value)?;
                let saved: serde_json::Value = self.lua.from_value(encoded)?;
                ensure!(
                    serde_json::to_vec(&saved)?.len() <= 8 * 1024 * 1024,
                    "reload state exceeds 8 MiB"
                );
                Some(saved)
            }
            _ => bail!("game.saveState must be a function or nil"),
        };
        let mut replacement = Self::load(fs, config, audio_enabled)?;
        replacement.extensions = self.extensions.clone();
        if let Some(saved) = saved {
            replacement.arm_watchdog();
            let replay: Table = replacement.lua.globals().get("replay")?;
            let decoded: Value = replay
                .get::<mlua::Function>("_decode")?
                .call(replacement.lua.to_value(&saved)?)?;
            let game: Table = replacement.lua.globals().get("game")?;
            ensure!(matches!(game.get::<Value>("restoreState")?, Value::Function(_)),
                "replacement must implement game.restoreState when the previous game has game.saveState");
            replacement.state.borrow_mut().loading = true;
            let result = replacement.call("restoreState", decoded);
            replacement.state.borrow_mut().loading = false;
            result?;
        }
        replacement.state.borrow_mut().input = input;
        replacement.state.borrow_mut().profile.reloads = reloads;
        Ok(replacement)
    }

    pub fn reload(&mut self, config: &Config, audio_enabled: bool) -> Result<()> {
        let fs = self.state.try_borrow()?.fs.clone();
        *self = self.replacement(fs, config, audio_enabled)?;
        Ok(())
    }

    pub fn add_extension(
        &self,
        extension: impl kairo_core::extensions::NativeExtension + 'static,
    ) -> Result<()> {
        let mut extensions = self.extensions.try_borrow_mut()?;
        ensure!(
            extensions.len() < 64 && !extensions.iter().any(|e| e.name() == extension.name()),
            "duplicate native extension or extension limit reached"
        );
        extensions.push(Box::new(extension));
        Ok(())
    }

    pub fn debug_command(&self, command: kairo_core::profiler::DebugCommand) -> Result<()> {
        anyhow::ensure!(
            self.state.borrow().development_tools,
            "runtime tools are disabled in this profile"
        );
        use kairo_core::profiler::{DebugCommand, SceneOperation};
        command.validate()?;
        self.arm_watchdog();
        match command {
            DebugCommand::Bookmark { label, note } => {
                let replay: Table = self.lua.globals().get("replay")?;
                let id: u64 = replay
                    .get::<mlua::Function>("bookmark")?
                    .call((label, note))?;
                log::info!("[replay] Created bug bookmark {id}");
                Ok(())
            }
            DebugCommand::Inspect { update } => inspector::apply(&self.lua, *update),
            DebugCommand::RuntimePage { session, offset } => {
                ensure!(
                    session == self.state.borrow().inspection_session,
                    "stale runtime session after reload"
                );
                self.state.borrow_mut().runtime_offset = offset;
                Ok(())
            }
            DebugCommand::RuntimeSelect { session, key } => {
                ensure!(
                    session == self.state.borrow().inspection_session,
                    "stale runtime session after reload"
                );
                let scenes: Table = self.lua.globals().get("scene")?;
                let valid: bool = scenes
                    .get::<mlua::Function>("_runtimeContains")?
                    .call((key.graph, key.id))?;
                ensure!(
                    valid,
                    "runtime node is stale or belongs to another active scene"
                );
                self.state.borrow_mut().runtime_selected = Some(key);
                Ok(())
            }
            DebugCommand::RuntimeEdit { key, update } => {
                ensure!(
                    update.session == self.state.borrow().inspection_session,
                    "stale runtime session after reload"
                );
                let scenes: Table = self.lua.globals().get("scene")?;
                scenes.get::<mlua::Function>("_runtimeEdit")?.call::<()>((
                    key.graph,
                    key.id,
                    self.lua.to_value(&*update)?,
                ))?;
                Ok(())
            }
            DebugCommand::Scene {
                session,
                operation,
                name,
            } => {
                ensure!(
                    session == self.state.borrow().inspection_session,
                    "stale scene session after reload"
                );
                let scenes: Table = self.lua.globals().get("scene")?;
                let method = match operation {
                    SceneOperation::Switch => "switch",
                    SceneOperation::Push => "push",
                    SceneOperation::Pop => "pop",
                    SceneOperation::Reload => "reload",
                };
                scenes.get::<mlua::Function>(method)?.call::<()>(name)?;
                scenes.get::<mlua::Function>("_flush")?.call::<()>(())?;
                Ok(())
            }
            DebugCommand::DebugDraw {
                physics,
                bounds,
                velocities,
                origins,
                names,
                camera,
            } => {
                let mut state = self.state.borrow_mut();
                state.debug_draw = kairo_core::debug_draw::DebugDraw {
                    physics,
                    bounds,
                    velocities,
                    origins,
                    names,
                    camera,
                };
                Ok(())
            }
            DebugCommand::Mixer {
                bus,
                volume,
                muted,
                fade,
                stop,
            } => {
                let mut state = self.state.borrow_mut();
                state.audio.set_bus(&bus, volume, muted, fade)?;
                if stop {
                    state.audio.stop_bus(&bus)?;
                }
                Ok(())
            }
            DebugCommand::Language { language } => {
                let localization: Table = self.lua.globals().get("localization")?;
                localization
                    .get::<mlua::Function>("setLanguage")?
                    .call::<()>(language)?;
                Ok(())
            }
            command => replay::command(&self.state, command),
        }
    }

    pub fn telemetry(&self) -> kairo_core::profiler::Telemetry {
        use kairo_core::profiler::{ReplayStatus, Telemetry};
        self.arm_watchdog();
        let (id, offset, selected) = {
            let state = self.state.borrow();
            (
                state.inspection_session,
                state.runtime_offset,
                state.runtime_selected,
            )
        };
        let inspection =
            inspector::snapshot(&self.lua, id, offset, selected).unwrap_or_else(|error| {
                let error: String = format!("{error:#}").chars().take(240).collect();
                kairo_core::inspector::InspectSnapshot {
                    session: id,
                    error: Some(error),
                    ..Default::default()
                }
            });
        let state = self.state.borrow();
        let infos = state.replay.timeline.infos();
        let mut profile = state.profile.for_telemetry();
        profile.active_nodes = inspection.scenes.active_nodes;
        Telemetry {
            inspection,
            mixer: state.audio.mixer(),
            tools_enabled: state.development_tools,
            bookmarks: state.replay.bookmarks.infos(),
            debug_draw: state.debug_draw,
            profile,
            replay: ReplayStatus {
                recording: state.replay.enabled,
                paused: state.replay.paused,
                frame: state.frame_number,
                time: state.elapsed,
                oldest: infos.first().map_or(state.elapsed, |info| info.time),
                newest: infos.last().map_or(state.elapsed, |info| info.time),
                snapshots: infos.len(),
                bytes: state.replay.timeline.bytes(),
            },
        }
    }

    pub fn ui_pointer(&self, down: bool, x: f32, y: f32, button: u8) -> Result<()> {
        self.arm_watchdog();
        ui::call(&self.lua, "_pointer", (down, x, y, button)).context("Kairo UI input error")?;
        let ui: Table = self.lua.globals().get("ui")?;
        let scene: Table = self.lua.globals().get("scene")?;
        if ui.get::<mlua::Function>("_covers")?.call::<bool>((x, y))? {
            return scene
                .get::<mlua::Function>("_cancelPointer")?
                .call::<()>(())
                .context("Kairo scene UI input cancellation error");
        }
        scene
            .get::<mlua::Function>("_pointer")?
            .call::<()>((down, x, y, button))
            .context("Kairo scene UI input error")
    }

    pub fn ui_cancel_pointer(&self) -> Result<()> {
        self.arm_watchdog();
        ui::call(&self.lua, "_cancelPointer", ()).context("Kairo UI input cancellation error")?;
        let scene: Table = self.lua.globals().get("scene")?;
        scene
            .get::<mlua::Function>("_cancelPointer")?
            .call::<()>(())
            .context("Kairo scene UI input cancellation error")
    }

    pub fn state(&self) -> &SharedState {
        &self.state
    }

    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    fn arm_watchdog(&self) {
        self.deadline.set(Instant::now() + Duration::from_secs(2));
    }

    pub fn call<A: IntoLuaMulti>(&self, name: &str, arguments: A) -> Result<()> {
        self.arm_watchdog();
        let arguments = arguments.into_lua_multi(&self.lua)?;
        let game: Table = self
            .lua
            .globals()
            .get("game")
            .context("global 'game' must be a table")?;
        match game.get::<Value>(name)? {
            Value::Nil => {}
            Value::Function(function) => {
                let profiling = self.state.borrow().profile.profiling_enabled;
                let start = profiling.then(Instant::now);
                let result = function.call::<()>(arguments.clone());
                if let Some(start) = start {
                    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
                    let mut state = self.state.borrow_mut();
                    state.profile.game_ms += elapsed_ms;
                    state
                        .profile
                        .record_callback("main.lua", "", "", name, elapsed_ms);
                }
                result.with_context(|| format!("Kairo2D Lua Error in game.{name}"))?;
            }
            _ => bail!("game.{name} must be a function or nil"),
        }
        let scenes: Table = self.lua.globals().get("scene")?;
        let mut forwarded = arguments;
        forwarded.push_front(Value::String(self.lua.create_string(name)?));
        let start = self
            .state
            .borrow()
            .profile
            .profiling_enabled
            .then(Instant::now);
        let result = scenes
            .get::<mlua::Function>("_dispatch")?
            .call::<()>(forwarded)
            .with_context(|| format!("Kairo2D Lua Error in scene.{name}"));
        if let Some(start) = start {
            self.state.borrow_mut().profile.scene_ms += start.elapsed().as_secs_f64() * 1000.0;
        }
        result
    }

    pub fn tick(&self, dt: f32) -> Result<()> {
        kairo_core::finite("delta time", &[dt])?;
        ensure!(dt >= 0.0, "delta time cannot be negative");
        let tick_start = self
            .state
            .borrow()
            .profile
            .profiling_enabled
            .then(Instant::now);
        self.arm_watchdog();
        self.state.borrow_mut().profile.begin_frame();
        replay::before(&self.lua)?;
        let advancing = {
            let mut state = self.state.borrow_mut();
            !state.replay.paused || std::mem::take(&mut state.replay.step)
        };
        let frame_dt = dt;
        let dt = if advancing { dt.min(0.1) } else { 0.0 };
        {
            let mut state = self
                .state
                .try_borrow_mut()
                .context("engine state is already in use")?;
            state.delta = dt;
            state.elapsed += f64::from(dt);
            if frame_dt > 0.0 {
                state.fps = if state.fps == 0.0 {
                    1.0 / frame_dt
                } else {
                    state.fps * 0.9 + 0.1 / frame_dt
                };
            }
        }
        {
            let mut state = self.state.borrow_mut();
            let input = state.input.clone();
            state.actions.sample(&input);
        }
        self.arm_watchdog();
        let ui_start = self
            .state
            .borrow()
            .profile
            .profiling_enabled
            .then(Instant::now);
        let ui_result = ui::call(&self.lua, "_update", ());
        if let Some(start) = ui_start {
            self.state.borrow_mut().profile.ui_ms += start.elapsed().as_secs_f64() * 1000.0;
        }
        ui_result?;
        let update_start = Instant::now();
        if advancing {
            let state = self.state.borrow();
            for extension in self.extensions.borrow_mut().iter_mut() {
                extension
                    .update(dt, &state.input)
                    .with_context(|| format!("native extension {} update", extension.name()))?;
            }
            drop(state);
            self.call("update", dt)?;
        }
        let update_ms = update_start.elapsed().as_secs_f64() * 1000.0;
        let scenes: Table = self.lua.globals().get("scene")?;
        let physics_paused: bool = scenes.get::<mlua::Function>("_physicsPaused")?.call(())?;
        if advancing {
            self.arm_watchdog();
            scenes
                .get::<mlua::Function>("_nativeUpdate")?
                .call::<()>(())?;
        }
        if advancing && !physics_paused {
            self.arm_watchdog();
            scenes
                .get::<mlua::Function>("_nativeBeforePhysics")?
                .call::<()>(())?;
        }
        {
            let mut state = self
                .state
                .try_borrow_mut()
                .context("engine state is already in use")?;
            let physics_start = Instant::now();
            if advancing {
                if !physics_paused {
                    state.physics.step(dt)?;
                }
                state.frame_number = state
                    .frame_number
                    .checked_add(1)
                    .context("frame counter exhausted")?;
            }
            state.profile.physics_ms = physics_start.elapsed().as_secs_f64() * 1000.0;
            state.profile.update_ms = update_ms;
            state.profile.frame_ms = f64::from(frame_dt) * 1000.0;
            state.profile.fps = state.fps;
            let audio_start = state.profile.profiling_enabled.then(Instant::now);
            state.audio.prune();
            if let Some(start) = audio_start {
                state.profile.audio_ms += start.elapsed().as_secs_f64() * 1000.0;
            }
            state.profile.active_bodies = state.physics.body_count();
            state.profile.active_colliders = state.physics.collider_count();
            state.profile.active_particles = state.particle_live_count.get();
            state.profile.active_emitters = state.particle_count.get();
            state.profile.audio_voices = state.audio.voice_count();
            state.profile.loaded_audio = state.audio.sound_count();
            state.frame.reset();
            state.color = Color::WHITE;
            state.camera = Camera::default();
            state.viewport = None;
            state.scissor = None;
            state.drawing = false;
        }
        if advancing && !physics_paused {
            self.arm_watchdog();
            scenes
                .get::<mlua::Function>("_nativeAfterPhysics")?
                .call::<()>(())?;
        }
        if advancing {
            self.arm_watchdog();
            replay::after(&self.lua, dt)?;
        }
        self.state.borrow_mut().drawing = true;
        let draw_start = Instant::now();
        let result = self.call("draw", ()).and_then(|()| {
            {
                let mut state = self.state.borrow_mut();
                if state.debug_draw.physics {
                    let segments = state.physics.debug_segments(state.debug_draw.velocities);
                    let camera = state.camera;
                    for segment in segments {
                        segment.draw(&mut state.frame, camera)?;
                    }
                }
                if state.debug_draw.bounds {
                    kairo_core::debug_draw::sprite_bounds(&mut state.frame)?;
                }
            }
            if !self.extensions.borrow().is_empty() {
                let mut state = self.state.borrow_mut();
                let size = [state.width, state.height];
                state.frame.push(kairo_core::DrawCommand::Viewport(None))?;
                state.frame.push(kairo_core::DrawCommand::Scissor(None))?;
                let mut canvas = kairo_core::extensions::Canvas::new(&mut state.frame, size);
                for extension in self.extensions.borrow_mut().iter_mut() {
                    extension
                        .draw(&mut canvas)
                        .with_context(|| format!("native extension {} draw", extension.name()))?;
                }
            }
            self.arm_watchdog();
            let ui_start = self
                .state
                .borrow()
                .profile
                .profiling_enabled
                .then(Instant::now);
            let ui_result = ui::call(&self.lua, "_draw", ()).context("Kairo UI draw error");
            if let Some(start) = ui_start {
                self.state.borrow_mut().profile.ui_ms += start.elapsed().as_secs_f64() * 1000.0;
            }
            ui_result
        });
        let mut state = self
            .state
            .try_borrow_mut()
            .context("engine state is already in use")?;
        state.drawing = false;
        state.profile.draw_ms = draw_start.elapsed().as_secs_f64() * 1000.0;
        state.profile.commands = state.frame.commands.len();
        state.profile.sprites_submitted = state
            .frame
            .commands
            .iter()
            .filter(|command| {
                matches!(command,
                    kairo_core::DrawCommand::Quad(quad) if quad.texture.is_some()
                ) || matches!(command,
                    kairo_core::DrawCommand::AffineQuad { quad, .. } if quad.texture.is_some()
                )
            })
            .count();
        state.profile.textures = state.assets.texture_count();
        state.profile.texture_bytes = state.assets.texture_bytes();
        if result.is_ok() {
            profiler::overlay(&mut state)?;
        }
        if result.is_err() {
            state.frame.commands.clear();
        }
        drop(state);
        result?;
        {
            let mut state = self.state.borrow_mut();
            state.debug_ui.windows.clear();
            state.debug_ui.current = None;
            state.debug_ui.building = true;
        }
        let result = if self.state.borrow().development_tools {
            self.call("debugUI", ())
        } else {
            Ok(())
        };
        let mut state = self.state.borrow_mut();
        state.debug_ui.building = false;
        state.debug_ui.responses.clear();
        state.input.clear_edges();
        if result.is_err() {
            state.debug_ui.windows.clear();
        }
        if let Some(start) = tick_start {
            state.profile.tick_ms = start.elapsed().as_secs_f64() * 1000.0;
        }
        result
    }
}

fn create_lua() -> mlua::Result<Lua> {
    let libraries = StdLib::TABLE
        | StdLib::STRING
        | StdLib::MATH
        | StdLib::UTF8
        | StdLib::COROUTINE
        | StdLib::PACKAGE;
    let lua = Lua::new_with(libraries, LuaOptions::default())?;
    lua.set_memory_limit(128 * 1024 * 1024)?;
    Ok(lua)
}

/// Compile all project Lua sources without executing game code or opening devices.
pub fn check_project(fs: &ProjectFs) -> Result<usize> {
    Config::load(fs)?;
    let lua = create_lua()?;
    lua.load(fs.read_text("main.lua")?)
        .set_name("@main.lua")
        .set_mode(ChunkMode::Text)
        .into_function()
        .context("Kairo2D Lua Error")?;
    let mut count = 0;
    check_directory(&lua, fs, fs.root(), 0, &mut count)?;
    Ok(count)
}

fn check_directory(
    lua: &Lua,
    fs: &ProjectFs,
    directory: &Path,
    depth: usize,
    count: &mut usize,
) -> Result<()> {
    ensure!(depth <= 64, "project directory nesting exceeds 64 levels");
    let mut entries = std::fs::read_dir(directory)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            continue;
        }
        let path = entry.path();
        if kind.is_dir() {
            if matches!(
                entry.file_name().to_str(),
                Some(".git" | ".github" | ".kairo" | "target" | "dist")
            ) {
                continue;
            }
            check_directory(lua, fs, &path, depth + 1, count)?;
        } else if path.extension().is_some_and(|extension| extension == "lua") {
            ensure!(*count < 10_000, "project contains too many Lua files");
            let relative = path
                .strip_prefix(fs.root())?
                .to_str()
                .context("Lua paths must be UTF-8")?
                .replace('\\', "/");
            lua.load(fs.read_text(&relative)?)
                .set_name(format!("@{relative}"))
                .set_mode(ChunkMode::Text)
                .into_function()
                .with_context(|| format!("Kairo2D Lua Error in {relative}"))?;
            *count += 1;
        }
    }
    Ok(())
}
