use crate::{control::ShutdownSignal, input, watch::ScriptWatcher, RunOptions};
use anyhow::{Context, Result};
use kairo_assets::AssetManager;
use kairo_core::{Camera, Color, Config, DrawCommand, Frame, ProjectFs};
use kairo_lua::{GameSession, WindowCommand};
use kairo_render::Renderer;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

pub fn run(fs: ProjectFs, config: Config, options: RunOptions) -> Result<()> {
    let watching = config.development.hot_reload && !options.no_watch && options.link.is_none();
    let link = match (&options.link, &options.link_token_file) {
        (Some(address), Some(token)) => {
            log::warn!(
                "[link] Remote development enabled: only connect to a trusted project author"
            );
            Some(kairo_link::Client::connect(
                address.clone(),
                kairo_link::SessionKey::read(token)?,
            )?)
        }
        _ => None,
    };
    let watcher = if watching {
        Some(ScriptWatcher::new(fs.root())?)
    } else {
        None
    };
    let mut app = App {
        log_tap: if link.is_some() {
            Some(crate::log_tap::listen())
        } else {
            None
        },
        link,
        _link_stage: None,
        link_status: String::new(),
        error_assets: AssetManager::new(fs.clone()),
        fs,
        config,
        profile: kairo_core::profile::BuildProfile::parse(&options.profile)?,
        gamepads: crate::gamepads::GamepadBackend::default(),
        strict: (!watching && options.link.is_none()) || options.frames.is_some(),
        audio_enabled: !options.no_audio,
        frame_limit: options.frames,
        watcher,
        window: None,
        renderer: None,
        session: None,
        game_error: None,
        fatal: None,
        last_frame: Instant::now(),
        last_telemetry: Instant::now(),
        frames: 0,
        minimized: false,
        reload_warning: None,
        shutdown: if options.controlled {
            Some(ShutdownSignal::listen()?)
        } else {
            None
        },
    };
    let event_loop = EventLoop::new().context("cannot create the window event loop")?;
    event_loop.run_app(&mut app)?;
    match app.fatal {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

struct App {
    fs: ProjectFs,
    link: Option<kairo_link::Client>,
    _link_stage: Option<tempfile::TempDir>,
    link_status: String,
    log_tap: Option<std::sync::mpsc::Receiver<String>>,
    gamepads: crate::gamepads::GamepadBackend,
    config: Config,
    profile: kairo_core::profile::BuildProfile,
    strict: bool,
    audio_enabled: bool,
    frame_limit: Option<u64>,
    watcher: Option<ScriptWatcher>,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    session: Option<GameSession>,
    error_assets: AssetManager,
    game_error: Option<String>,
    fatal: Option<anyhow::Error>,
    last_frame: Instant,
    frames: u64,
    last_telemetry: Instant,
    minimized: bool,
    reload_warning: Option<String>,
    shutdown: Option<ShutdownSignal>,
}

impl App {
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: anyhow::Error) {
        self.fatal = Some(error);
        event_loop.exit();
    }

    fn game_failed(&mut self, event_loop: &ActiveEventLoop, error: anyhow::Error) {
        if self.strict {
            self.fail(event_loop, error);
            return;
        }
        let message = format!("{error:#}");
        log::error!("{message}\nEdit a Lua file or press F5 to restart the game.");
        if let Some(session) = &self.session {
            session.state().borrow_mut().audio.stop_all();
        }
        self.game_error = Some(message);
    }

    fn reload(&mut self, event_loop: &ActiveEventLoop) {
        let mut config = self.config.clone();
        if let Some(window) = &self.window {
            let size = window.inner_size();
            // game.load must observe the current viewport, including after a resize.
            config.game.width = size.width.max(1);
            config.game.height = size.height.max(1);
        }
        let result = if let Some(session) = &mut self.session {
            session.reload(&config, self.audio_enabled)
        } else {
            GameSession::load(self.fs.clone(), &config, self.audio_enabled).map(|session| {
                self.session = Some(session);
            })
        };
        match result {
            Ok(()) => {
                self.game_error = None;
                self.reload_warning = None;
                if let Some(renderer) = &mut self.renderer {
                    renderer.clear_assets();
                }
                log::info!("Game session ready (F5 restarts it)");
            }
            Err(error) if self.session.is_some() && !self.strict => {
                let message = format!("{error:#}");
                log::warn!("Hot reload failed; keeping previous session\n{message}");
                self.reload_warning = Some(message);
            }
            Err(error) => self.game_failed(event_loop, error),
        }
        self.last_frame = Instant::now();
    }

    fn poll_link(&mut self) {
        let Some(link) = &self.link else {
            return;
        };
        if let Some(receiver) = &self.log_tap {
            for line in receiver.try_iter().take(128) {
                link.log(&line);
            }
        }
        let status = link.status();
        if status != self.link_status {
            log::info!("[link] {status}");
            self.link_status = status;
        }
        for envelope in link.take_commands() {
            let result = self
                .session
                .as_ref()
                .context("runtime session is not ready")
                .and_then(|session| session.debug_command(envelope.command));
            link.command_result(envelope.id, result);
        }
        let Some(bundle) = link.take_update() else {
            return;
        };
        let revision = bundle.revision.clone();
        let result = (|| -> Result<()> {
            let staged = bundle.stage()?;
            let fs = ProjectFs::new(staged.path())?;
            kairo_lua::check_project(&fs)?;
            let mut config = Config::load_profile(&fs, self.profile)?;
            anyhow::ensure!(
                config.save.identity == self.config.save.identity,
                "Link cannot change the local save identity; restart explicitly to switch projects"
            );
            anyhow::ensure!(
                config.micro == self.config.micro,
                "Link cannot change the Micro render profile; restart explicitly"
            );
            if let Some(window) = &self.window {
                let size = window.inner_size();
                config.game.width = size.width.max(1);
                config.game.height = size.height.max(1);
            }
            let candidate = match &self.session {
                Some(session) => session.replacement(fs.clone(), &config, self.audio_enabled)?,
                None => GameSession::load(fs.clone(), &config, self.audio_enabled)?,
            };
            self.session = Some(candidate);
            self.fs = fs;
            self.config = config;
            self._link_stage = Some(staged);
            self.game_error = None;
            self.reload_warning = None;
            if let Some(renderer) = &mut self.renderer {
                renderer.clear_assets();
                renderer.set_vsync(self.config.game.vsync);
            }
            if let Some(window) = &self.window {
                window.set_title(&self.config.game.title);
            }
            self.last_frame = Instant::now();
            Ok(())
        })();
        let accepted = result.is_ok();
        let message = match result {
            Ok(()) => "Candidate loaded and committed; explicit state hooks applied when provided"
                .to_owned(),
            Err(error) => {
                let text = format!("{error:#}");
                self.reload_warning = Some(text.clone());
                text
            }
        };
        if accepted {
            log::info!("[link] Applied revision {}", &revision[..12]);
        } else {
            log::warn!("[link] Rejected candidate; previous session retained: {message}");
        }
        if let Some(link) = &self.link {
            link.report(kairo_link::ReloadReport {
                revision,
                accepted,
                message,
            });
        }
    }

    fn invoke(&mut self, event_loop: &ActiveEventLoop, f: impl FnOnce(&GameSession) -> Result<()>) {
        if self.game_error.is_some() || self.fatal.is_some() {
            return;
        }
        let result = self.session.as_ref().map(f).unwrap_or(Ok(()));
        if let Err(error) = result {
            self.game_failed(event_loop, error);
        }
    }

    fn resize(&mut self, event_loop: &ActiveEventLoop, size: PhysicalSize<u32>) -> Result<()> {
        self.minimized = size.width == 0 || size.height == 0;
        self.last_frame = Instant::now();
        if let Some(renderer) = &mut self.renderer {
            renderer.resize(size.width, size.height)?;
        }
        if let Some(session) = &self.session {
            let mut state = session.state().borrow_mut();
            state.physical_size = [size.width, size.height];
            state.width = if state.micro.enabled {
                state.micro.width
            } else {
                size.width
            };
            state.height = if state.micro.enabled {
                state.micro.height
            } else {
                size.height
            };
        }
        if !self.minimized {
            self.invoke(event_loop, |session| {
                let state = session.state().borrow();
                let dimensions = (state.width, state.height);
                drop(state);
                session.call("resized", dimensions)
            });
        }
        Ok(())
    }

    fn window_commands(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let commands = match &self.session {
            Some(session) => std::mem::take(&mut session.state().borrow_mut().window_commands),
            None => return Ok(()),
        };
        for command in commands {
            match command {
                WindowCommand::Close => event_loop.exit(),
                WindowCommand::Title(title) => {
                    if let Some(window) = &self.window {
                        window.set_title(&title);
                    }
                }
                WindowCommand::Size(width, height) => {
                    if let Some(window) = &self.window {
                        if let Some(size) =
                            window.request_inner_size(PhysicalSize::new(width, height))
                        {
                            self.resize(event_loop, size)?;
                        }
                    }
                }
                WindowCommand::Vsync(enabled) => {
                    if let Some(renderer) = &mut self.renderer {
                        renderer.set_vsync(enabled);
                    }
                }
            }
        }
        Ok(())
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        if self.minimized || self.fatal.is_some() {
            return Ok(());
        }
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        let pad_events = if let Some(session) = &self.session {
            self.gamepads
                .poll(&mut session.state().borrow_mut().input.gamepads)
        } else {
            Vec::new()
        };
        for event in pad_events {
            use crate::gamepads::PadEvent;
            self.invoke(event_loop, |session| match event {
                PadEvent::Connected(id, name) => session.call("gamepadConnected", (id, name)),
                PadEvent::Disconnected(id) => session.call("gamepadDisconnected", id),
                PadEvent::Button(id, name, down) => {
                    if down {
                        session
                            .state()
                            .borrow_mut()
                            .input
                            .mark_gamepad_pressed(id, name);
                    }
                    session.call(
                        if down {
                            "gamepadPressed"
                        } else {
                            "gamepadReleased"
                        },
                        (id, name),
                    )
                }
            });
        }
        self.invoke(event_loop, |session| session.tick(dt));
        if self.fatal.is_some() {
            return Ok(());
        }
        if self.game_error.is_none() {
            self.window_commands(event_loop)?;
        }
        let Some(renderer) = &mut self.renderer else {
            return Ok(());
        };
        if let Some(message) = &self.game_error {
            let size = self
                .window
                .as_ref()
                .map(|window| window.inner_size())
                .unwrap_or(PhysicalSize::new(960, 540));
            let (width, height) = if self.config.micro.enabled {
                (self.config.micro.width, self.config.micro.height)
            } else {
                (size.width, size.height)
            };
            let frame = error_frame(message, width, height);
            renderer.render(&frame, &self.error_assets)?;
        } else if let Some(session) = &self.session {
            let mut state = session.state().borrow_mut();
            let render_start = state.profile.profiling_enabled.then(Instant::now);
            let stats = if let Some(warning) = &self.reload_warning {
                let mut frame = state.frame.clone();
                frame.commands.push(DrawCommand::Viewport(None));
                frame.commands.push(DrawCommand::Scissor(None));
                frame.commands.push(DrawCommand::Quad(kairo_core::Quad {
                    texture: None,
                    size: [state.width as f32, 52.0].into(),
                    uv_min: [0.0, 0.0].into(),
                    uv_max: [1.0, 1.0].into(),
                    transform: Default::default(),
                    camera: Camera::default(),
                    color: Color([0.1, 0.03, 0.02, 0.96]),
                }));
                let detail: String = warning
                    .lines()
                    .next()
                    .unwrap_or("See editor console")
                    .chars()
                    .take(100)
                    .collect();
                frame.commands.push(DrawCommand::Text {
                    text: format!("RELOAD FAILED - previous game retained\n{detail}\nFix the file and save, or press F5"),
                    position: [8.0, 8.0].into(), scale: 1.0, camera: Camera::default(), color: Color::WHITE,
                });
                renderer.render(&frame, &state.assets)?
            } else {
                let state = &mut *state;
                renderer.render_with_debug(
                    &state.frame,
                    &state.assets,
                    Some(&mut state.debug_ui),
                )?
            };
            state.profile.batches = stats.draw_calls;
            state.profile.vertices = stats.vertices;
            if let Some(start) = render_start {
                state.profile.render_ms = start.elapsed().as_secs_f64() * 1000.0;
            }
        }
        if (self.shutdown.is_some() || self.link.is_some())
            && self.last_telemetry.elapsed() >= Duration::from_millis(200)
        {
            if let Some(session) = &self.session {
                let sample = session.telemetry();
                if let Some(link) = &self.link {
                    if let Err(error) = link.update_telemetry(sample.clone()) {
                        log::warn!("[inspector] {error:#}");
                    }
                }
                if self.shutdown.is_some() {
                    if let Ok(json) = serde_json::to_string(&sample) {
                        use std::io::Write;
                        let _ = writeln!(std::io::stdout().lock(), "@kairo:telemetry {json}");
                    }
                }
            }
            self.last_telemetry = Instant::now();
        }
        self.frames += 1;
        if self.frame_limit.is_some_and(|limit| self.frames >= limit) {
            event_loop.exit();
        }
        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title(&self.config.game.title)
            .with_inner_size(PhysicalSize::new(
                self.config.game.width,
                self.config.game.height,
            ))
            .with_resizable(self.config.game.resizable);
        let result = (|| -> Result<()> {
            let window = Arc::new(
                event_loop
                    .create_window(attributes)
                    .context("cannot open a window")?,
            );
            let mut renderer =
                pollster::block_on(Renderer::new(window.clone(), self.config.game.vsync))?;
            renderer.configure_micro(&self.config.micro)?;
            self.window = Some(window);
            self.renderer = Some(renderer);
            Ok(())
        })();
        match result {
            Ok(()) => self.reload(event_loop),
            Err(error) => self.fail(event_loop, error),
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if self.window.as_ref().is_none_or(|window| window.id() != id) {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Err(error) = self.resize(event_loop, size) {
                    self.fail(event_loop, error);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.redraw(event_loop) {
                    self.fail(event_loop, error);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.repeat {
                    return;
                }
                let PhysicalKey::Code(code) = event.physical_key else {
                    return;
                };
                let down = event.state == ElementState::Pressed;
                if code == KeyCode::F11 && self.config.development.tools {
                    if down {
                        if let Some(session) = &self.session {
                            if let Err(error) = session.debug_command(
                                kairo_core::profiler::DebugCommand::Bookmark {
                                    label: "Bug bookmark".into(),
                                    note: String::new(),
                                },
                            ) {
                                log::warn!("[replay] {error:#}");
                            }
                        }
                    }
                    return;
                }
                if code == KeyCode::F5 {
                    if down {
                        self.reload(event_loop);
                    }
                    return;
                }
                let Some(key) = input::key_name(code) else {
                    return;
                };
                let changed = self
                    .session
                    .as_ref()
                    .is_some_and(|session| session.state().borrow_mut().input.set_key(key, down));
                if changed {
                    let callback = if down { "keyPressed" } else { "keyReleased" };
                    self.invoke(event_loop, |session| session.call(callback, key));
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.debug_pointer(position.x as f32, position.y as f32);
                }
                let mut delta = (0.0, 0.0);
                let mut point = [position.x as f32, position.y as f32];
                if let Some(session) = &self.session {
                    let mut state = session.state().borrow_mut();
                    if state.micro.enabled {
                        let canvas = [state.width, state.height];
                        let viewport = kairo_core::micro::viewport(
                            state.physical_size,
                            canvas,
                            state.micro.integer_scaling,
                        );
                        point = kairo_core::micro::pointer(point, viewport, canvas);
                    }
                    delta = (
                        point[0] - state.input.mouse_x,
                        point[1] - state.input.mouse_y,
                    );
                    state.input.mouse_x = point[0];
                    state.input.mouse_y = point[1];
                }
                self.invoke(event_loop, |session| {
                    session.call("mouseMoved", (point[0], point[1], delta.0, delta.1))
                });
            }
            WindowEvent::MouseInput {
                state: pressed,
                button,
                ..
            } => {
                let Some(button) = input::button_number(button) else {
                    return;
                };
                let down = pressed == ElementState::Pressed;
                if let Some(renderer) = &mut self.renderer {
                    renderer.debug_button(button, down);
                }
                let event = self.session.as_ref().and_then(|session| {
                    let mut state = session.state().borrow_mut();
                    state.input.set_button(button, down).then_some((
                        state.input.mouse_x,
                        state.input.mouse_y,
                        button,
                    ))
                });
                if let Some(arguments) = event {
                    let callback = if down {
                        "mousePressed"
                    } else {
                        "mouseReleased"
                    };
                    self.invoke(event_loop, |session| {
                        session.ui_pointer(down, arguments.0, arguments.1, arguments.2)?;
                        session.call(callback, arguments)
                    });
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (x, y) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (x, y),
                    MouseScrollDelta::PixelDelta(position) => {
                        (position.x as f32 / 40.0, position.y as f32 / 40.0)
                    }
                };
                self.invoke(event_loop, |session| session.call("wheelMoved", (x, y)));
            }
            WindowEvent::Focused(focused) => {
                if !focused {
                    if let Some(renderer) = &mut self.renderer {
                        renderer.debug_focus_lost();
                    }
                    self.invoke(event_loop, GameSession::ui_cancel_pointer);
                }
                if !focused {
                    let releases = self.session.as_ref().map(|session| {
                        let mut state = session.state().borrow_mut();
                        (
                            state.input.release_all(),
                            state.input.mouse_x,
                            state.input.mouse_y,
                        )
                    });
                    if let Some(((keys, buttons), x, y)) = releases {
                        for key in keys {
                            self.invoke(event_loop, |session| session.call("keyReleased", key));
                        }
                        for button in buttons {
                            self.invoke(event_loop, |session| {
                                session.call("mouseReleased", (x, y, button))
                            });
                        }
                    }
                }
                self.invoke(event_loop, |session| session.call("focusChanged", focused));
            }
            _ => {}
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(session) = &self.session {
            if let Err(error) = session.call("quit", ()) {
                log::error!("{error:#}");
            }
            session.state().borrow_mut().audio.stop_all();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.fatal.is_some() {
            return;
        }
        if self
            .shutdown
            .as_ref()
            .is_some_and(ShutdownSignal::requested)
        {
            event_loop.exit();
            return;
        }
        self.poll_link();
        let commands = self
            .shutdown
            .as_ref()
            .map(ShutdownSignal::drain)
            .unwrap_or_default();
        for command in commands {
            if let Some(session) = &self.session {
                if let Err(error) = session.debug_command(command) {
                    log::warn!("Debugger: {error:#}");
                }
            }
        }
        if let Some(changes) = self.watcher.as_mut().and_then(ScriptWatcher::ready) {
            if changes.scripts {
                log::info!("Lua files changed; preparing replacement session");
                self.reload(event_loop);
            } else if let Some(session) = &self.session {
                let mut success = false;
                let mut failure = None;
                for path in changes.textures {
                    match session.state().borrow_mut().assets.reload_texture(&path) {
                        Ok(true) => {
                            success = true;
                            log::info!("Reloaded texture {}", path.display());
                        }
                        Ok(false) => {}
                        Err(error) => {
                            let message =
                                format!("Texture reload failed: {}: {error:#}", path.display());
                            log::warn!("{message}; keeping previous pixels");
                            failure = Some(message);
                        }
                    }
                }
                if let Some(message) = failure {
                    self.reload_warning = Some(message);
                } else if success
                    && self
                        .reload_warning
                        .as_ref()
                        .is_some_and(|message| message.starts_with("Texture reload failed"))
                {
                    self.reload_warning = None;
                }
            }
        }
        let interval = Duration::from_millis(33);
        let now = Instant::now();
        let idle = self.game_error.is_some() || self.minimized;
        let redraw_due = !idle || now.duration_since(self.last_frame) >= interval;
        event_loop.set_control_flow(if idle {
            let deadline = if self.minimized || redraw_due {
                now + interval
            } else {
                self.last_frame + interval
            };
            ControlFlow::WaitUntil(deadline)
        } else {
            ControlFlow::Poll
        });
        if !self.minimized && redraw_due {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }
}

fn error_frame(message: &str, width: u32, height: u32) -> Frame {
    let mut frame = Frame {
        clear: Color([0.12, 0.035, 0.045, 1.0]),
        ..Default::default()
    };
    let scale = if width >= 720 { 2.0 } else { 1.0 };
    let columns = ((width.saturating_sub(40) as f32 / (8.0 * scale)) as usize).max(8);
    let rows = ((height.saturating_sub(40) as f32 / (10.0 * scale)) as usize).max(1);
    let source = format!("KAIRO2D - GAME PAUSED\nF5: restart | save Lua: reload\n\n{message}");
    let mut lines = Vec::new();
    for line in source.lines() {
        let characters: Vec<char> = line.chars().collect();
        if characters.is_empty() {
            lines.push(String::new());
        }
        for chunk in characters.chunks(columns) {
            lines.push(chunk.iter().collect::<String>());
        }
    }
    lines.truncate(rows);
    frame.commands.push(DrawCommand::Text {
        text: lines.join("\n"),
        position: [20.0, 20.0].into(),
        scale,
        camera: Camera::default(),
        color: Color::WHITE,
    });
    frame
}
