use anyhow::{ensure, Result};
use kairo_assets::AssetManager;
use kairo_audio::AudioEngine;
use kairo_core::{Camera, Color, Config, Frame, InputState, PhysicsWorld, ProjectFs};
use std::cell::RefCell;
use std::rc::Rc;

pub type SharedState = Rc<RefCell<EngineState>>;

#[derive(Debug)]
pub enum WindowCommand {
    Title(String),
    Size(u32, u32),
    Vsync(bool),
    Close,
}

pub struct EngineState {
    pub assets: AssetManager,
    pub fonts: kairo_assets::fonts::FontManager,
    pub(crate) tilemaps:
        std::collections::HashMap<std::path::PathBuf, Rc<kairo_assets::tilemap::TileMap>>,
    pub audio: AudioEngine,
    pub physics: PhysicsWorld,
    pub input: InputState,
    pub debug_draw: kairo_core::debug_draw::DebugDraw,
    pub particle_count: Rc<std::cell::Cell<usize>>,
    pub inspection_session: u64,
    pub actions: kairo_core::actions::Actions,
    pub frame: Frame,
    pub width: u32,
    pub micro: kairo_core::config::MicroConfig,
    pub physical_size: [u32; 2],
    pub height: u32,
    pub delta: f32,
    pub elapsed: f64,
    pub fps: f32,
    pub frame_number: u64,
    pub debug_ui: kairo_core::debugui::DebugUi,
    pub profile: kairo_core::profiler::ProfileSample,
    pub show_profiler: bool,
    pub development_tools: bool,
    pub replay: crate::replay::ReplayState,
    pub(crate) rng: kairo_replay::Rng,
    pub window_commands: Vec<WindowCommand>,
    pub(crate) fs: ProjectFs,
    pub(crate) save_identity: String,
    pub(crate) loading: bool,
    pub(crate) color: Color,
    pub(crate) camera: Camera,
    pub(crate) drawing: bool,
    pub(crate) viewport: Option<kairo_core::PixelRect>,
    pub(crate) scissor: Option<kairo_core::PixelRect>,
}

impl EngineState {
    pub fn new(fs: ProjectFs, config: &Config, audio_enabled: bool) -> Result<Self> {
        config.validate()?;
        let mut actions = kairo_core::actions::Actions::default();
        if fs.exists("input.toml")? {
            actions.replace(kairo_core::actions::InputBindings::parse(
                &fs.read_text("input.toml")?,
            )?)?;
        }
        Ok(Self {
            assets: AssetManager::new(fs.clone()),
            fonts: kairo_assets::fonts::FontManager::new(fs.clone()),
            tilemaps: std::collections::HashMap::new(),
            audio: AudioEngine::new(fs.clone(), config.audio.enabled && audio_enabled),
            physics: PhysicsWorld::new(&config.physics)?,
            input: InputState::default(),
            actions,
            debug_draw: Default::default(),
            particle_count: Rc::new(std::cell::Cell::new(0)),
            inspection_session: kairo_core::inspector::next_session()?,
            frame: Frame::default(),
            width: if config.micro.enabled {
                config.micro.width
            } else {
                config.game.width
            },
            height: if config.micro.enabled {
                config.micro.height
            } else {
                config.game.height
            },
            physical_size: [config.game.width, config.game.height],
            micro: config.micro.clone(),
            delta: 0.0,
            elapsed: 0.0,
            fps: 0.0,
            frame_number: 0,
            debug_ui: Default::default(),
            profile: Default::default(),
            show_profiler: config.development.profiler,
            development_tools: config.development.tools,
            replay: crate::replay::ReplayState::new(&config.replay)?,
            rng: kairo_replay::Rng::default(),
            window_commands: Vec::new(),
            fs,
            save_identity: config.save.identity.clone(),
            loading: true,
            color: Color::WHITE,
            camera: Camera::default(),
            drawing: false,
            viewport: None,
            scissor: None,
        })
    }

    pub(crate) fn require_draw(&self) -> Result<()> {
        ensure!(self.drawing, "drawing is only allowed inside game.draw");
        Ok(())
    }

    pub(crate) fn window_command(&mut self, command: WindowCommand) -> Result<()> {
        ensure!(
            self.window_commands.len() < 128,
            "too many pending window commands"
        );
        self.window_commands.push(command);
        Ok(())
    }
}

pub(crate) fn with_state<T>(
    state: &SharedState,
    f: impl FnOnce(&mut EngineState) -> Result<T>,
) -> mlua::Result<T> {
    let mut state = state
        .try_borrow_mut()
        .map_err(|_| mlua::Error::RuntimeError("engine state is already in use".into()))?;
    f(&mut state).map_err(lua_error)
}

pub(crate) fn lua_error(error: anyhow::Error) -> mlua::Error {
    mlua::Error::RuntimeError(format!("{error:#}"))
}

pub(crate) fn check_options(table: &mlua::Table, allowed: &[&str]) -> mlua::Result<()> {
    for pair in table.clone().pairs::<mlua::Value, mlua::Value>() {
        let (key, _) = pair?;
        let mlua::Value::String(key) = key else {
            return Err(mlua::Error::RuntimeError(
                "option names must be strings".into(),
            ));
        };
        let key = key.to_str()?;
        if !allowed.contains(&key.as_ref()) {
            return Err(mlua::Error::RuntimeError(format!("unknown option '{key}'")));
        }
    }
    Ok(())
}
