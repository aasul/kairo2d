use crate::code::CodeTools;
use crate::console::Console;
use crate::dialogs::{Dialog, FileOperation};
use crate::feature_settings::SettingsPage;
use crate::features::FeatureStatus;
use crate::hub::{Hub, HubAction};
use crate::link::LinkConnection;
use crate::process::{self, Operation, Process};
use crate::tabs::Tab;
use crate::workspace::{BrowserAction, Workspace};
use anyhow::{ensure, Result};
use eframe::egui::{self, Color32, Key, Modifiers, RichText};
use kairo_project::{SettingsDocument, Template};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum ProfilerSort {
    #[default]
    Total,
    Average,
    Maximum,
    Calls,
}

#[derive(Clone)]
pub(crate) enum Action {
    Hub,
    OpenDialog,
    Open(PathBuf),
    CloseTab(usize),
    Save,
    SaveAll,
    SaveCopy,
    Reload,
    NewSprite,
    NewAnimation,
    Settings,
    Export,
    Build(PathBuf),
    Run,
    Stop,
    Restart,
    Check,
    Preferences,
    Api,
    Quit,
    Overview,
    Micro,
    Link,
    Replay,
    ReplaySettings,
    StartReplay,
    Profiler,
    UiGuide,
    ApiQuery(String),
    OpenFile(PathBuf),
    NewProject(Template),
    HostLink(std::net::SocketAddr),
    ConnectLink(LinkConnection),
    Tool(crate::develop::ToolTab),
    InspectPeer(u64),
    Palette,
    OpenLocation(PathBuf, usize),
    Control {
        peer: u64,
        command: Box<kairo_core::profiler::DebugCommand>,
    },
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct Preferences {
    pub recent: Vec<PathBuf>,
    pub font_size: f32,
    pub project_width: f32,
    pub inspector_width: f32,
    pub console_height: f32,
    pub runtime: Option<PathBuf>,
    pub overview_on_open: bool,
    pub profile: kairo_core::profile::BuildProfile,
    pub last_tabs: std::collections::BTreeMap<PathBuf, Vec<PathBuf>>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            recent: Vec::new(),
            font_size: 15.0,
            project_width: 230.0,
            inspector_width: 240.0,
            console_height: 170.0,
            runtime: None,
            overview_on_open: true,
            profile: Default::default(),
            last_tabs: Default::default(),
        }
    }
}

pub struct KairoApp {
    pub(crate) preferences: Preferences,
    pub(crate) workspace: Option<Workspace>,
    pub(crate) dialog: Option<Dialog>,
    pub(crate) error: Option<String>,
    pub(crate) console: Console,
    pub(crate) show_api: bool,
    pub(crate) api_query: String,
    pub(crate) develop: crate::develop::DevelopTools,
    palette: crate::commands::CommandPalette,
    hub: Hub,
    code_tools: CodeTools,
    process: Option<Process>,
    run_link: Option<LinkConnection>,
    runtime_status: Result<PathBuf, String>,
    show_overview: bool,
    settings_pending_restart: bool,
    telemetry: Option<kairo_core::profiler::Telemetry>,
    show_profiler: bool,
    profiler_sort: ProfilerSort,
    profile_history: VecDeque<f64>,
    show_replay: bool,
    bookmark_panel: crate::bookmark_panel::BookmarkPanel,
    show_link: bool,
    link: crate::link::LinkTools,
    restart_pending: bool,
    exit_allowed: bool,
    last_refresh: Instant,
}

impl KairoApp {
    pub fn new(cc: &eframe::CreationContext<'_>, project: Option<PathBuf>) -> Self {
        let mut preferences: Preferences = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, "preferences"))
            .unwrap_or_default();
        if !preferences.font_size.is_finite() {
            preferences.font_size = 15.0;
        }
        preferences.font_size = preferences.font_size.clamp(10.0, 28.0);
        for (value, fallback) in [
            (&mut preferences.project_width, 230.0),
            (&mut preferences.inspector_width, 240.0),
            (&mut preferences.console_height, 170.0),
        ] {
            if !value.is_finite() || *value < 40.0 || *value > 2000.0 {
                *value = fallback;
            }
        }
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.panel_fill = Color32::from_rgb(24, 27, 33);
        style.visuals.window_fill = Color32::from_rgb(29, 33, 40);
        style.visuals.extreme_bg_color = Color32::from_rgb(18, 21, 27);
        style.visuals.selection.bg_fill = Color32::from_rgb(48, 74, 103);
        style.visuals.selection.stroke.color = Color32::from_rgb(206, 224, 243);
        style.spacing.item_spacing = egui::vec2(8.0, 7.0);
        cc.egui_ctx.set_style(style);
        let runtime_status = process::runtime_path(preferences.runtime.as_deref())
            .map_err(|error| format!("{error:#}"));
        let mut app = Self {
            preferences,
            workspace: None,
            dialog: None,
            error: None,
            console: Console::default(),
            show_api: false,
            api_query: String::new(),
            develop: Default::default(),
            palette: Default::default(),
            hub: Hub::default(),
            code_tools: CodeTools::default(),
            process: None,
            run_link: None,
            runtime_status,
            show_overview: true,
            settings_pending_restart: false,
            bookmark_panel: Default::default(),
            telemetry: None,
            show_profiler: false,
            profiler_sort: ProfilerSort::default(),
            profile_history: VecDeque::new(),
            show_replay: false,
            show_link: false,
            link: Default::default(),
            restart_pending: false,
            exit_allowed: false,
            last_refresh: Instant::now(),
        };
        if let Some(path) = project {
            if let Err(error) = app.open_project(path) {
                app.report(error);
            }
        }
        app
    }

    pub(crate) fn report(&mut self, error: anyhow::Error) {
        let message = format!("{error:#}");
        self.console.push(format!("[ERROR] {message}"));
        self.error = Some(message);
    }

    fn open_project(&mut self, path: PathBuf) -> Result<()> {
        self.capture_tabs();
        let mut workspace = Workspace::open(&path)?;
        if let Some(paths) = self.preferences.last_tabs.get(workspace.files.root()) {
            for path in paths.iter().take(16) {
                if let Err(error) = workspace.open_file(path.clone()) {
                    self.console.push(format!(
                        "[WARN] Could not reopen {}: {error}",
                        path.display()
                    ));
                }
            }
        }
        let root = workspace.files.root().to_owned();
        self.link.reset();
        self.develop = Default::default();
        self.palette.open = false;
        self.process = None;
        self.run_link = None;
        self.telemetry = None;
        self.profile_history.clear();
        self.settings_pending_restart = false;
        self.show_overview = self.preferences.overview_on_open;
        self.restart_pending = false;
        self.preferences.recent.retain(|item| *item != root);
        self.preferences.recent.insert(0, root.clone());
        self.preferences.recent.truncate(15);
        self.workspace = Some(workspace);
        self.console
            .push(format!("[INFO] Opened {}", root.display()));
        Ok(())
    }

    pub(crate) fn perform(&mut self, action: Action, ctx: &egui::Context) {
        let result = self.execute(action, ctx, false);
        if let Err(error) = result {
            self.report(error);
        }
    }

    pub(crate) fn execute(
        &mut self,
        action: Action,
        ctx: &egui::Context,
        confirmed: bool,
    ) -> Result<()> {
        let dirty = self
            .workspace
            .as_ref()
            .is_some_and(|workspace| match &action {
                Action::CloseTab(index) => workspace.tabs.get(*index).is_some_and(Tab::dirty),
                Action::Hub
                | Action::OpenDialog
                | Action::Open(_)
                | Action::NewProject(_)
                | Action::Quit => workspace.dirty(),
                _ => false,
            });
        let dirty = dirty
            || (self.develop.dirty()
                && matches!(
                    &action,
                    Action::Hub
                        | Action::OpenDialog
                        | Action::Open(_)
                        | Action::NewProject(_)
                        | Action::Quit
                ));
        if dirty && !confirmed {
            self.dialog = Some(Dialog::Unsaved(action));
            return Ok(());
        }
        match action {
            Action::Tool(tab) => {
                ensure!(
                    self.workspace.is_some(),
                    "open a project before using development tools"
                );
                self.develop.tab = tab;
                self.develop.open = true;
            }
            Action::InspectPeer(peer) => {
                self.develop.target = peer;
                self.develop.tab = crate::develop::ToolTab::Inspector;
                self.develop.open = true;
            }
            Action::Palette => self.palette.show(),
            Action::OpenLocation(path, line) => {
                if let Some(workspace) = &mut self.workspace {
                    workspace.open_file(path)?;
                    if let Some(Tab::Code(doc)) = workspace.tabs.get_mut(workspace.active) {
                        doc.goto_line(line);
                    }
                    self.show_overview = false;
                }
            }
            Action::Control { peer, command } => {
                if peer == 0 {
                    ensure!(self.running_game(), "start the local runtime first");
                    if let Some(process) = &mut self.process {
                        process.debug_command(&command)?;
                    }
                } else {
                    let id = self.link.command(peer, *command)?;
                    self.console.push(format!("[INFO][inspector] Queued command {id} for tester {peer}; waiting for acknowledgement"));
                }
            }
            Action::Hub => self.close_project(),
            Action::NewProject(template) => {
                self.close_project();
                self.hub.select_template(template);
            }
            Action::Overview => {
                self.show_overview = true;
                self.show_link = false;
                self.show_replay = false;
                self.show_profiler = false;
            }
            Action::Micro => {
                if self.workspace.is_some() {
                    self.open_settings(SettingsPage::Micro)?;
                } else {
                    self.hub.select_template(Template::Micro);
                }
            }
            Action::ReplaySettings => self.open_settings(SettingsPage::Replay)?,
            Action::Link => {
                self.show_link = true;
                self.show_replay = false;
                self.show_profiler = false;
            }
            Action::Replay => {
                self.show_replay = true;
                self.show_link = false;
                self.show_profiler = false;
            }
            Action::Profiler => {
                self.show_profiler = true;
                self.show_link = false;
                self.show_replay = false;
            }
            Action::UiGuide => self.dialog = Some(Dialog::UiGuide),
            Action::ApiQuery(query) => {
                self.api_query = query;
                self.show_api = true;
            }
            Action::OpenFile(path) => {
                if let Some(workspace) = &mut self.workspace {
                    workspace.open_file(path)?;
                    self.show_overview = false;
                }
            }
            Action::StartReplay => {
                ensure!(
                    self.process.is_none(),
                    "stop the active operation before starting Replay"
                );
                let workspace = self
                    .workspace
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("open a project first"))?;
                let mut settings = SettingsDocument::load(&workspace.files)?;
                settings.config.replay.enabled = true;
                workspace.save_settings(&mut settings)?;
                self.run_link = None;
                self.show_replay = true;
                self.launch(Operation::Run, None)?;
            }
            Action::HostLink(bind) => {
                let workspace = self
                    .workspace
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("open a project first"))?;
                self.develop.save(workspace)?;
                workspace.save_all()?;
                self.link.start(workspace.files.root(), bind)?;
                self.show_link = true;
                self.console.push("[INFO][link] Hosting saved project; share the private token with trusted testers only");
            }
            Action::ConnectLink(mut connection) => {
                ensure!(
                    self.process.is_none(),
                    "stop the active operation before joining Link"
                );
                let workspace = self
                    .workspace
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("open a project first"))?;
                connection.validate(workspace.files.root())?;
                connection.token_file = connection.token_file.canonicalize()?;
                self.run_link = Some(connection);
                if let Err(error) = self.launch(Operation::Run, None) {
                    self.run_link = None;
                    return Err(error);
                }
                self.show_link = true;
            }
            Action::OpenDialog => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Open Kairo project folder")
                    .pick_folder()
                {
                    self.open_project(path)?;
                }
            }
            Action::Open(path) => self.open_project(path)?,
            Action::CloseTab(index) => {
                if let Some(workspace) = &mut self.workspace {
                    workspace.close_tab(index);
                }
            }
            Action::Save => {
                if let Some(workspace) = &mut self.workspace {
                    if let Some(tab) = workspace.tabs.get_mut(workspace.active) {
                        if tab.dirty() {
                            tab.save(&workspace.files)?;
                            self.console
                                .push(format!("[INFO] Saved {}", tab.path().display()));
                        }
                    }
                    workspace.refresh()?;
                }
            }
            Action::SaveAll => {
                if let Some(workspace) = &mut self.workspace {
                    self.develop.save(workspace)?;
                    workspace.save_all()?;
                    self.console.push("[INFO] Saved all modified files");
                }
            }
            Action::SaveCopy => {
                if let Some(workspace) = &self.workspace {
                    if let Some(tab) = workspace.tabs.get(workspace.active) {
                        let stem = tab.path().file_stem().unwrap_or_default().to_string_lossy();
                        let extension =
                            tab.path().extension().unwrap_or_default().to_string_lossy();
                        let path = tab
                            .path()
                            .with_file_name(format!("{stem}-copy.{extension}"));
                        self.dialog = Some(Dialog::File {
                            operation: FileOperation::SaveCopy(workspace.active),
                            value: path.display().to_string(),
                        });
                    }
                }
            }
            Action::Reload => {
                if let Some(workspace) = &mut self.workspace {
                    if let Some(tab) = workspace.tabs.get_mut(workspace.active) {
                        ensure!(
                            !tab.dirty(),
                            "save or close the modified tab before reloading"
                        );
                        *tab = Tab::open(&workspace.files, tab.path().to_owned())?;
                    }
                }
            }
            Action::NewSprite => {
                if self.workspace.is_some() {
                    self.dialog = Some(Dialog::Sprite {
                        path: "assets/sprite.png".into(),
                        width: 32,
                        height: 32,
                    });
                }
            }
            Action::NewAnimation => {
                let workspace = self
                    .workspace
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("open a project"))?;
                let sheet = if let Some(selected) = workspace
                    .selected
                    .as_ref()
                    .filter(|path| path.extension().is_some_and(|e| e == "png"))
                {
                    selected.clone()
                } else {
                    let Some(path) = rfd::FileDialog::new()
                        .set_title("Choose a PNG sprite sheet inside this project")
                        .set_directory(workspace.files.root())
                        .add_filter("PNG", &["png"])
                        .pick_file()
                    else {
                        return Ok(());
                    };
                    path.canonicalize()?
                        .strip_prefix(workspace.files.root())
                        .map_err(|_| anyhow::anyhow!("select a PNG inside the current project"))?
                        .to_owned()
                };
                let path = sheet.with_extension("anim.json");
                if workspace.files.resolve(&path)?.try_exists()? {
                    workspace.open_file(path)?;
                } else if let Some(index) = workspace.tabs.iter().position(|tab| tab.path() == path)
                {
                    workspace.active = index;
                } else {
                    ensure!(
                        workspace.tabs.len() < 32,
                        "close a tab before opening another animation"
                    );
                    let editor = crate::animation::AnimationEditor::create(
                        &workspace.files,
                        path,
                        &sheet.to_string_lossy(),
                    )?;
                    workspace.tabs.push(Tab::Animation(Box::new(editor)));
                    workspace.active = workspace.tabs.len() - 1;
                }
                self.show_overview = false;
            }
            Action::Settings => self.open_settings(SettingsPage::General)?,
            Action::Export => {
                if let Some(workspace) = &self.workspace {
                    self.dialog = Some(Dialog::Export {
                        output: workspace
                            .files
                            .root()
                            .join("dist/game")
                            .display()
                            .to_string(),
                    });
                }
            }
            Action::Build(output) => self.launch(Operation::Build, Some(output))?,
            Action::Run => {
                ensure!(self.process.is_none(), "stop the active operation first");
                self.run_link = None;
                self.launch(Operation::Run, None)?;
            }
            Action::Check => self.launch(Operation::Check, None)?,
            Action::Stop => {
                self.restart_pending = false;
                if let Some(process) = &mut self.process {
                    process.stop();
                }
            }
            Action::Restart => {
                if let Some(process) = &mut self.process {
                    ensure!(
                        process.operation == Operation::Run,
                        "stop the active Check / Build before restarting a game"
                    );
                    process.stop();
                    self.restart_pending = true;
                } else {
                    self.launch(Operation::Run, None)?;
                }
            }
            Action::Preferences => self.dialog = Some(Dialog::Preferences),
            Action::Api => self.show_api = !self.show_api,
            Action::Quit => {
                self.link.stop();
                self.exit_allowed = true;
                self.process = None;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        Ok(())
    }

    fn close_project(&mut self) {
        self.capture_tabs();
        self.link.reset();
        self.develop = Default::default();
        self.palette.open = false;
        self.process = None;
        self.run_link = None;
        self.workspace = None;
        self.telemetry = None;
        self.profile_history.clear();
        self.show_link = false;
        self.show_replay = false;
        self.show_profiler = false;
        self.show_api = false;
        self.restart_pending = false;
        self.settings_pending_restart = false;
    }

    fn capture_tabs(&mut self) {
        if let Some(workspace) = &self.workspace {
            let mut paths: Vec<_> = workspace
                .tabs
                .iter()
                .map(|tab| tab.path().to_owned())
                .collect();
            if let Some(tab) = workspace.tabs.get(workspace.active) {
                paths.retain(|path| path != tab.path());
                paths.push(tab.path().to_owned());
            }
            self.preferences
                .last_tabs
                .insert(workspace.files.root().to_owned(), paths);
        }
        self.preferences
            .last_tabs
            .retain(|root, _| self.preferences.recent.contains(root));
    }

    pub(crate) fn focus_editor(&mut self) {
        self.show_overview = false;
    }

    pub(crate) fn running_game(&self) -> bool {
        self.process
            .as_ref()
            .is_some_and(|process| process.operation == Operation::Run)
    }

    pub(crate) fn operation_active(&self) -> bool {
        self.process.is_some()
    }

    pub(crate) fn refresh_runtime_status(&mut self) {
        self.runtime_status = process::runtime_path(self.preferences.runtime.as_deref())
            .map_err(|error| format!("{error:#}"));
    }

    fn open_settings(&mut self, page: SettingsPage) -> Result<()> {
        let workspace = self
            .workspace
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("open a project first"))?;
        self.dialog = Some(Dialog::Settings {
            settings: Box::new(SettingsDocument::load(&workspace.files)?),
            page,
        });
        Ok(())
    }

    pub(crate) fn save_project_settings(&mut self, settings: &mut SettingsDocument) -> Result<()> {
        let workspace = self
            .workspace
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("open a project first"))?;
        workspace.save_settings(settings)?;
        self.settings_pending_restart = self.running_game();
        self.console
            .push("[INFO] Project settings saved; apply them with Run / Restart");
        Ok(())
    }

    fn feature_status(&self) -> FeatureStatus {
        let running = self.running_game();
        FeatureStatus {
            busy: self.process.is_some(),
            running,
            recording: running
                && self
                    .telemetry
                    .as_ref()
                    .is_some_and(|sample| sample.replay.recording),
            paused: running
                && self
                    .telemetry
                    .as_ref()
                    .is_some_and(|sample| sample.replay.paused),
            link: if running && self.run_link.is_some() {
                "TESTER RUNNING".into()
            } else {
                self.link.status()
            },
            runtime_ready: self.runtime_status.is_ok(),
        }
    }

    fn launch(&mut self, operation: Operation, output: Option<PathBuf>) -> Result<()> {
        ensure!(
            self.process.is_none(),
            "stop the active run/check/build first"
        );
        let runtime = process::runtime_path(self.preferences.runtime.as_deref())?;
        let workspace = self
            .workspace
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("open a project first"))?;
        self.develop.save(workspace)?;
        workspace.save_all()?;
        let mut arguments: Vec<OsString> = match operation {
            Operation::Run => vec!["run".into(), ".".into(), "--controlled".into()],
            Operation::Check => vec!["check".into(), ".".into()],
            Operation::Build => vec!["build".into(), ".".into()],
        };
        if matches!(operation, Operation::Run | Operation::Build) {
            arguments.extend(["--profile".into(), self.preferences.profile.name().into()]);
        }
        if operation == Operation::Run {
            if let Some(connection) = &self.run_link {
                connection.validate(workspace.files.root())?;
                arguments.extend(connection.arguments());
            }
        }
        if let Some(output) = output {
            arguments.push("--output".into());
            arguments.push(output.into_os_string());
        }
        self.telemetry = None;
        self.profile_history.clear();
        self.process = Some(Process::start(
            &runtime,
            workspace.files.root(),
            &arguments,
            operation,
        )?);
        self.settings_pending_restart = false;
        self.console.push(format!("[INFO] Starting {operation:?}"));
        Ok(())
    }

    fn debug_panels(&mut self, ctx: &egui::Context) {
        use kairo_core::profiler::DebugCommand;
        let sample = self.telemetry.clone();
        let running = self.running_game();
        let project_open = self.workspace.is_some();
        let can_start = project_open && self.process.is_none();
        let mut command = None;
        let mut action = None;
        let profiler_sort = &mut self.profiler_sort;
        let profile_history = &self.profile_history;
        if self.show_profiler {
            egui::Window::new("Profiler")
                .open(&mut self.show_profiler)
                .default_width(430.0)
                .vscroll(true)
                .show(ctx, |ui| {
                    ui.label("CPU and rendering statistics from the local runtime");
                    if !running {
                        ui.label("Start a game to collect live samples.");
                        if ui
                            .add_enabled(can_start, egui::Button::new("Run game"))
                            .clicked()
                        {
                            action = Some(Action::Run);
                        }
                    }
                    if let Some(sample) = &sample {
                        let p = &sample.profile;
                        ui.label(format!("{:.0} FPS  |  {:.2} ms frame interval", p.fps, p.frame_ms));
                        if profile_history.len() > 1 {
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width().max(120.0), 56.0),
                                egui::Sense::hover(),
                            );
                            let ceiling = profile_history.iter().copied().fold(16.67_f64, f64::max);
                            let points = profile_history.iter().enumerate().map(|(index, ms)| {
                                egui::pos2(
                                    rect.left() + rect.width() * index as f32 / (profile_history.len() - 1) as f32,
                                    rect.bottom() - rect.height() * (*ms / ceiling).clamp(0.0, 1.0) as f32,
                                )
                            }).collect();
                            ui.painter().add(egui::Shape::line(points, egui::Stroke::new(1.5_f32, Color32::LIGHT_GREEN)));
                        }
                        egui::Grid::new("profile-values")
                            .num_columns(2)
                            .show(ui, |ui| {
                                for (name, value) in [
                                    ("Simulation CPU", format!("{:.3} ms", p.tick_ms)),
                                    ("Update total", format!("{:.3} ms", p.update_ms)),
                                    ("Draw / game UI", format!("{:.3} ms", p.draw_ms)),
                                    ("Physics", format!("{:.3} ms", p.physics_ms)),
                                    ("Game callbacks", format!("{:.3} ms", p.game_ms)),
                                    ("Scene dispatch", format!("{:.3} ms", p.scene_ms)),
                                    ("UI", format!("{:.3} ms", p.ui_ms)),
                                    ("Audio maintenance", format!("{:.3} ms", p.audio_ms)),
                                    ("Renderer call", format!("{:.3} ms", p.render_ms)),
                                    ("Draw commands", p.commands.to_string()),
                                    ("Draw calls", p.batches.to_string()),
                                    ("Sprites submitted", p.sprites_submitted.to_string()),
                                    ("Vertices", p.vertices.to_string()),
                                    ("Lua callbacks", p.lua_callbacks.to_string()),
                                    ("Physics bodies", p.active_bodies.to_string()),
                                    ("Active nodes", p.active_nodes.to_string()),
                                    ("Colliders", p.active_colliders.to_string()),
                                    ("Active particles", p.active_particles.to_string()),
                                    ("Particle emitters", p.active_emitters.to_string()),
                                    ("Audio voices", p.audio_voices.to_string()),
                                    ("Loaded sounds", p.loaded_audio.to_string()),
                                    ("Textures", p.textures.to_string()),
                                    (
                                        "Decoded texture memory",
                                        format!("{:.2} MiB", p.texture_bytes as f64 / 1_048_576.0),
                                    ),
                                    ("Session reloads", p.reloads.to_string()),
                                ] {
                                    ui.label(name);
                                    ui.monospace(value);
                                    ui.end_row();
                                }
                            });
                        if p.profiling_enabled {
                            ui.separator();
                            ui.strong("Lua callbacks in sampled frame");
                            ui.horizontal(|ui| {
                                ui.label("Sort:");
                                for (label, sort) in [
                                    ("Total", ProfilerSort::Total),
                                    ("Average", ProfilerSort::Average),
                                    ("Max", ProfilerSort::Maximum),
                                    ("Calls", ProfilerSort::Calls),
                                ] {
                                    ui.selectable_value(profiler_sort, sort, label);
                                }
                            });
                            let mut callbacks = p.callback_samples.clone();
                            callbacks.sort_by(|a, b| {
                                let order = match profiler_sort {
                                    ProfilerSort::Total => b.total_ms.total_cmp(&a.total_ms),
                                    ProfilerSort::Average => b.average_ms().total_cmp(&a.average_ms()),
                                    ProfilerSort::Maximum => b.max_ms.total_cmp(&a.max_ms),
                                    ProfilerSort::Calls => b.calls.cmp(&a.calls),
                                };
                                order.then_with(|| a.script.cmp(&b.script)).then_with(|| a.node_path.cmp(&b.node_path))
                            });
                            egui::Grid::new("lua-callback-profile").striped(true).show(ui, |ui| {
                                for heading in ["Callback", "Calls", "Total", "Avg", "Max"] { ui.strong(heading); }
                                ui.end_row();
                                for entry in &callbacks {
                                    ui.label(format!("{}:{} [{} / {}]", entry.script, entry.callback, entry.scene, entry.node_path));
                                    ui.monospace(entry.calls.to_string());
                                    ui.monospace(format!("{:.3}", entry.total_ms));
                                    ui.monospace(format!("{:.3}", entry.average_ms()));
                                    ui.monospace(format!("{:.3}", entry.max_ms));
                                    ui.end_row();
                                }
                            });
                            if p.callback_samples_dropped > 0 {
                                ui.small(format!("{} callback calls omitted from this sample; the busiest recorded callbacks are shown.", p.callback_samples_dropped));
                            }
                        } else {
                            ui.small("Choose the Debug build profile and restart the game to collect callback timings.");
                        }
                        if !running {
                            ui.label("Last sample, not live data.");
                        }
                    } else if running {
                        ui.label(
                            "Waiting for runtime telemetry. Inspect the Console if startup failed.",
                        );
                    }
                    ui.small("Samples arrive at 5 Hz. Timings are CPU wall time and may overlap; GPU time is not measured.");
                });
        }
        if self.show_replay {
            egui::Window::new("Replay").open(&mut self.show_replay).default_width(520.0).vscroll(true).show(ctx, |ui| {
                ui.label("Record / pause / restore - experimental");
                ui.horizontal_wrapped(|ui| {
                    if ui.add_enabled(project_open, egui::Button::new("Recording settings...")).clicked() { action = Some(Action::ReplaySettings); }
                    if ui.button("State registration snippets").clicked() { action = Some(Action::ApiQuery("replay.".into())); }
                });
                if !running {
                    ui.separator();
                    ui.label("The local game is stopped. Enable recording and launch it here, or run it with F5.");
                    if ui.add_enabled(can_start, egui::Button::new("Enable recording & Run")).clicked() { action = Some(Action::StartReplay); }
                    if ui.button("New Replay demo project").clicked() { action = Some(Action::NewProject(Template::Replay)); }
                } else if let Some(sample) = &sample {
                    let replay = &sample.replay;
                    let mut recording = replay.recording;
                    if ui.checkbox(&mut recording, "Record snapshots now").changed() {
                        command = Some(DebugCommand::Record { enabled: recording });
                    }
                    ui.small("This switch controls the current run. Recording settings control future runs.");
                    ui.label(format!("Frame {}  |  {:.2}s  |  {} snapshots  |  {:.2} MiB",
                        replay.frame, replay.time, replay.snapshots, replay.bytes as f64 / 1_048_576.0));
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(if replay.paused { "Resume (branch)" } else { "Pause" }).clicked() {
                            command = Some(if replay.paused { DebugCommand::Resume } else { DebugCommand::Pause });
                        }
                        if ui.button("Step one frame").clicked() { command = Some(DebugCommand::Step); }
                        if ui.add_enabled(replay.snapshots > 0, egui::Button::new("Latest snapshot")).clicked() {
                            command = Some(DebugCommand::Seek { time: replay.newest });
                        }
                    });
                    if replay.snapshots > 0 {
                        let mut time = replay.time.clamp(replay.oldest, replay.newest);
                        if ui.add(egui::Slider::new(&mut time, replay.oldest..=replay.newest).text("seconds")).changed() {
                            command = Some(DebugCommand::Seek { time });
                        }
                    } else {
                        ui.label("No snapshots yet. Turn on recording and register the Lua tables you want restored.");
                    }
                } else {
                    ui.label("Waiting for runtime telemetry. Check that this editor and runtime are from the same build.");
                }
                ui.separator();
                ui.label("In game.load, register your game state, for example:");
                ui.code("replay.register(\"player\", player)");
                ui.small("Only registered Lua state and supported runtime state are restored. Scrubbing pauses; Resume discards future snapshots. Closing this panel does not stop recording.");
                if running {if let Some(sample)=&sample {ui.separator();ui.add_enabled_ui(sample.tools_enabled,|ui|{
                    if let Some(next)=self.bookmark_panel.ui(ui,0,sample){command=Some(next);}
                });}}
            });
        }
        if let (Some(command), Some(process)) = (command, &mut self.process) {
            if let Err(error) = process.debug_command(&command) {
                self.report(error);
            }
        }
        if let Some(action) = action {
            self.perform(action, ctx);
        }
    }

    fn poll_process(&mut self) {
        let mut finished = false;
        if let Some(process) = &mut self.process {
            for line in process.drain() {
                if let Some(json) = line.strip_prefix("@kairo:telemetry ") {
                    if let Ok(sample) = serde_json::from_str(json) {
                        let sample: kairo_core::profiler::Telemetry = sample;
                        if sample.profile.frame_ms.is_finite() {
                            self.profile_history.push_back(sample.profile.frame_ms);
                            if self.profile_history.len() > 60 {
                                self.profile_history.pop_front();
                            }
                        }
                        self.telemetry = Some(sample);
                    }
                } else {
                    self.console.push(line);
                }
            }
            match process.poll() {
                Ok(Some(message)) => self.console.push(message),
                Ok(None) => {}
                Err(error) => {
                    self.console.push(format!("[ERROR] {error:#}"));
                    finished = true;
                }
            }
            finished |= process.finished();
            if finished {
                for line in process.drain() {
                    if let Some(json) = line.strip_prefix("@kairo:telemetry ") {
                        if let Ok(sample) = serde_json::from_str(json) {
                            let sample: kairo_core::profiler::Telemetry = sample;
                            if sample.profile.frame_ms.is_finite() {
                                self.profile_history.push_back(sample.profile.frame_ms);
                                if self.profile_history.len() > 60 {
                                    self.profile_history.pop_front();
                                }
                            }
                            self.telemetry = Some(sample);
                        }
                    } else {
                        self.console.push(line);
                    }
                }
            }
        }
        if finished {
            self.process = None;
            if !self.restart_pending {
                self.run_link = None;
            }
        }
        if self.process.is_none() && self.restart_pending {
            self.restart_pending = false;
            if let Err(error) = self.launch(Operation::Run, None) {
                self.report(error);
            }
        }
    }

    fn browser_action(&mut self, action: BrowserAction) -> Result<()> {
        let Some(workspace) = &mut self.workspace else {
            return Ok(());
        };
        match action {
            BrowserAction::Open(path) => {
                workspace.open_file(path)?;
                self.show_overview = false;
            }
            BrowserAction::Refresh => workspace.refresh()?,
            BrowserAction::NewFile => {
                self.dialog = Some(Dialog::File {
                    operation: FileOperation::NewFile,
                    value: workspace
                        .selected_parent()
                        .join("script.lua")
                        .display()
                        .to_string(),
                })
            }
            BrowserAction::NewFolder => {
                self.dialog = Some(Dialog::File {
                    operation: FileOperation::NewFolder,
                    value: workspace
                        .selected_parent()
                        .join("folder")
                        .display()
                        .to_string(),
                })
            }
            BrowserAction::Rename(path) => {
                let graph = kairo_project::dependencies::DependencyGraph::scan(&workspace.files)?;
                let preview = graph
                    .references()
                    .iter()
                    .filter(|reference| {
                        reference.target == path || reference.target.starts_with(&path)
                    })
                    .map(|reference| {
                        format!(
                            "{} {} → {}",
                            reference.owner.display(),
                            reference.field,
                            reference.target.display()
                        )
                    })
                    .collect();
                self.dialog = Some(Dialog::File {
                    operation: FileOperation::Rename {
                        from: path.clone(),
                        preview,
                    },
                    value: path.display().to_string(),
                })
            }
            BrowserAction::Duplicate(path) => {
                self.dialog = Some(Dialog::File {
                    operation: FileOperation::Duplicate(path.clone()),
                    value: path
                        .with_file_name(format!(
                            "copy-{}",
                            path.file_name().unwrap_or_default().to_string_lossy()
                        ))
                        .display()
                        .to_string(),
                })
            }
            BrowserAction::Delete(path) => self.dialog = Some(Dialog::Delete(path)),
            BrowserAction::Reveal => {
                let root = workspace.files.root();
                let command = if cfg!(windows) {
                    "explorer"
                } else if cfg!(target_os = "macos") {
                    "open"
                } else {
                    "xdg-open"
                };
                let mut child = std::process::Command::new(command).arg(root).spawn()?;
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
            }
        }
        Ok(())
    }

    fn toolbar(&mut self, ctx: &egui::Context) -> Option<Action> {
        let mut action = None;
        let enabled = self.dialog.is_none();
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            if !enabled {
                ui.disable();
            }
            egui::menu::bar(ui, |ui| {
                ui.label(RichText::new("Kairo").strong().size(18.0));
                ui.small(format!("{} | Workflow Tools", env!("CARGO_PKG_VERSION")));
                ui.menu_button("File", |ui| {
                    for (label, value) in [
                        ("Projects...", Action::Hub),
                        ("Open project...", Action::OpenDialog),
                        ("Save", Action::Save),
                        ("Save all", Action::SaveAll),
                        ("Save copy...", Action::SaveCopy),
                        ("Reload current file", Action::Reload),
                        ("Preferences...", Action::Preferences),
                        ("Quit", Action::Quit),
                    ] {
                        if ui.button(label).clicked() {
                            action = Some(value);
                            ui.close_menu();
                        }
                    }
                });
                let project_open = self.workspace.is_some();
                ui.add_enabled_ui(project_open, |ui| {
                    egui::ComboBox::from_id_source("build-profile")
                        .selected_text(self.preferences.profile.name())
                        .show_ui(ui, |ui| {
                            for profile in kairo_core::profile::BuildProfile::ALL {
                                ui.selectable_value(
                                    &mut self.preferences.profile,
                                    profile,
                                    profile.name(),
                                );
                            }
                        });
                    if ui
                        .add_enabled(self.process.is_none(), egui::Button::new("Run  F5"))
                        .clicked()
                    {
                        action = Some(Action::Run);
                    }
                    if ui
                        .add_enabled(self.process.is_some(), egui::Button::new("Stop"))
                        .clicked()
                    {
                        action = Some(Action::Stop);
                    }
                    if ui
                        .add_enabled(
                            self.process.is_none() || self.running_game(),
                            egui::Button::new("Restart"),
                        )
                        .clicked()
                    {
                        action = Some(Action::Restart);
                    }
                    ui.separator();
                    if ui
                        .add_enabled(self.process.is_none(), egui::Button::new("Check"))
                        .clicked()
                    {
                        action = Some(Action::Check);
                    }
                    if ui.button("Export...").clicked() {
                        action = Some(Action::Export);
                    }
                    ui.menu_button("Assets", |ui| {
                        if ui.button("New sprite...").clicked() {
                            action = Some(Action::NewSprite);
                            ui.close_menu();
                        }
                        if ui.button("Animate selected PNG").clicked() {
                            action = Some(Action::NewAnimation);
                            ui.close_menu();
                        }
                    });
                    ui.menu_button("Tools", |ui| {
                        if ui.button("Project settings...").clicked() {
                            action = Some(Action::Settings);
                            ui.close_menu();
                        }
                        if ui.button("Command palette...  Ctrl+Shift+P").clicked() {
                            action = Some(Action::Palette);
                            ui.close_menu();
                        }
                        for tab in crate::develop::ToolTab::ALL {
                            if ui.button(tab.name()).clicked() {
                                action = Some(Action::Tool(tab));
                                ui.close_menu();
                            }
                        }
                        if ui.button("API snippets...").clicked() {
                            action = Some(Action::Api);
                            ui.close_menu();
                        }
                        ui.separator();
                        for (label, value) in [
                            ("Features overview  F6", Action::Overview),
                            ("Fantasy Console  F7", Action::Micro),
                            ("Kairo Link  F8", Action::Link),
                            ("Replay  F9", Action::Replay),
                            ("Profiler  F10", Action::Profiler),
                            ("Lua UI setup", Action::UiGuide),
                        ] {
                            if ui.button(label).clicked() {
                                action = Some(value);
                                ui.close_menu();
                            }
                        }
                    });
                });
                if let Some(process) = &self.process {
                    ui.spinner();
                    ui.small(format!("{:?}", process.operation));
                }
            });
        });
        action
    }

    fn feature_bar(&mut self, ctx: &egui::Context) -> Option<Action> {
        let mut action = None;
        let status = self.feature_status();
        let micro = self
            .workspace
            .as_ref()
            .is_some_and(|workspace| workspace.config.micro.enabled);
        let replay_armed = !status.running
            && self
                .workspace
                .as_ref()
                .is_some_and(|workspace| workspace.config.replay.enabled);
        egui::TopBottomPanel::top("feature-bar").show(ctx, |ui| {
            ui.add_enabled_ui(self.dialog.is_none(), |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.small("FEATURES");
                    for (label, selected, value, tip) in [
                        ("Overview".to_owned(), self.show_overview && self.workspace.is_some(), Action::Overview, "Feature overview - F6"),
                        (format!("Fantasy Console: {}", if micro { "ON" } else { "OFF" }), micro, Action::Micro, "Configure Kairo Micro - F7. Saved settings apply on Run / Restart."),
                        (format!("Link: {}", status.link), self.show_link, Action::Link, "Host or join a trusted session - F8"),
                        (format!("Replay: {}", if status.paused { "PAUSED" } else if status.recording { "REC" } else if replay_armed { "READY" } else { "OFF" }), self.show_replay, Action::Replay, "Record / pause / restore snapshots - F9"),
                        ("Profiler".to_owned(), self.show_profiler, Action::Profiler, "Inspect CPU and renderer statistics - F10"),
                        ("Lua UI".to_owned(), false, Action::UiGuide, "Learn game UI and debugui; open snippets or a demo"),
                    ] {
                        if ui.selectable_label(selected, label).on_hover_text(tip).clicked() { action = Some(value); }
                    }
                    ui.separator();
                    for tab in crate::develop::ToolTab::ALL {
                        if ui.selectable_label(self.develop.open && self.develop.tab==tab,tab.name()).clicked(){action=Some(Action::Tool(tab));}
                    }
                    if ui.small_button("Commands...").on_hover_text("Ctrl/Cmd+Shift+P").clicked(){action=Some(Action::Palette);}
                    let (label, detail) = match &self.runtime_status {
                        Ok(path) => ("Runtime: found", path.display().to_string()),
                        Err(error) => ("Runtime: missing - locate...", error.clone()),
                    };
                    if ui.small_button(label).on_hover_text(detail).clicked() { action = Some(Action::Preferences); }
                });
                if self.settings_pending_restart && status.running {
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(Color32::from_rgb(233, 190, 112), "Project settings changed. The running game still uses its previous settings.");
                        if ui.button("Restart to apply").clicked() { action = Some(Action::Restart); }
                    });
                }
            });
        });
        action
    }

    fn shortcuts(&mut self, ctx: &egui::Context) -> Option<Action> {
        if self.dialog.is_some() {
            return None;
        }
        let mut action = None;
        ctx.input_mut(|input| {
            if input.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::S) {
                action = Some(Action::SaveAll);
            } else if input.consume_key(Modifiers::COMMAND, Key::S) {
                action = Some(if self.develop.open && self.develop.dirty() {
                    Action::SaveAll
                } else {
                    Action::Save
                });
            }
            if input.consume_key(Modifiers::SHIFT, Key::F5) {
                action = Some(Action::Stop);
            } else if input.consume_key(Modifiers::NONE, Key::F5) {
                action = Some(if self.process.is_some() {
                    Action::Restart
                } else {
                    Action::Run
                });
            }
            for (key, value) in [
                (Key::F6, Action::Overview),
                (Key::F7, Action::Micro),
                (Key::F8, Action::Link),
                (Key::F9, Action::Replay),
                (Key::F10, Action::Profiler),
            ] {
                if input.consume_key(Modifiers::NONE, key) {
                    action = Some(value);
                }
            }
            if input.consume_key(Modifiers::NONE, Key::F11) {
                action = Some(Action::Control {
                    peer: 0,
                    command: Box::new(kairo_core::profiler::DebugCommand::Bookmark {
                        label: "Bug bookmark".into(),
                        note: String::new(),
                    }),
                });
            }
            if input.consume_key(Modifiers::COMMAND, Key::O) {
                action = Some(Action::OpenDialog);
            }
            if input.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::P) {
                action = Some(Action::Palette);
            }
            if input.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::F) {
                action = Some(Action::Tool(crate::develop::ToolTab::Search));
            }
            if input.consume_key(Modifiers::COMMAND, Key::P) {
                self.dialog = Some(Dialog::QuickOpen(String::new()));
            }
            if input.consume_key(Modifiers::COMMAND, Key::F) {
                self.code_tools.find_open = true;
            }
            if input.consume_key(Modifiers::COMMAND, Key::H) {
                self.code_tools.find_open = true;
                self.code_tools.replace_open = true;
            }
            if input.consume_key(Modifiers::COMMAND, Key::G) {
                self.code_tools.goto_open = true;
            }
            if input.consume_key(Modifiers::CTRL, Key::Space) {
                action = Some(Action::Api);
            }
            if let Some(workspace) = &mut self.workspace {
                if let Some(Tab::Sprite(sprite)) = workspace.tabs.get_mut(workspace.active) {
                    if input.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z)
                        || input.consume_key(Modifiers::COMMAND, Key::Y)
                    {
                        sprite.pixels.redo();
                    } else if input.consume_key(Modifiers::COMMAND, Key::Z) {
                        sprite.pixels.undo();
                    }
                }
            }
        });
        action
    }

    fn workspace_ui(&mut self, ctx: &egui::Context) -> Option<Action> {
        let enabled = self.dialog.is_none();
        let mut action = None;
        let mut browser_action = None;
        let mut error = None;
        let feature_status = self.feature_status();
        let panel = egui::TopBottomPanel::bottom("console")
            .resizable(true)
            .default_height(self.preferences.console_height)
            .min_height(90.0)
            .show(ctx, |ui| {
                if !enabled {
                    ui.disable();
                }
                if let Some(location) = self.console.ui(ui) {
                    if let Some(workspace) = &mut self.workspace {
                        match workspace.open_file(location.path) {
                            Ok(()) => {
                                self.show_overview = false;
                                if let Some(Tab::Code(doc)) =
                                    workspace.tabs.get_mut(workspace.active)
                                {
                                    doc.goto_line(location.line);
                                }
                            }
                            Err(problem) => error = Some(problem),
                        }
                    }
                }
            });
        self.preferences.console_height = panel.response.rect.height();
        if let Some(workspace) = &mut self.workspace {
            let panel = egui::SidePanel::left("project-browser")
                .resizable(true)
                .default_width(self.preferences.project_width)
                .width_range(160.0..=480.0)
                .show(ctx, |ui| {
                    if !enabled {
                        ui.disable();
                    }
                    browser_action = workspace.browser(ui);
                });
            self.preferences.project_width = panel.response.rect.width();
            let panel = egui::SidePanel::right("inspector").resizable(true).default_width(self.preferences.inspector_width).width_range(170.0..=420.0).show(ctx, |ui| {
                if !enabled {
                    ui.disable();
                }
                ui.strong("INSPECTOR");
                ui.separator();
                ui.label(&workspace.config.game.title);
                ui.small(format!("{} x {}", workspace.config.game.width, workspace.config.game.height));
                if let Some(path) = &workspace.selected {
                    ui.add_space(14.0);
                    ui.label(path.display().to_string());
                    if let Some(node) = workspace.nodes.iter().find(|node| node.relative == *path) {
                        if !node.directory {
                            ui.small(format!("{} bytes", node.bytes));
                            if ui.button("Open file").clicked() { browser_action = Some(BrowserAction::Open(path.clone())); }
                        }
                    }
                }
                ui.add_space(18.0);
                ui.small("F5  Run / Restart\nShift+F5  Stop\nCtrl/Cmd+S  Save\nCtrl/Cmd+Shift+S  Save all\nCtrl/Cmd+F  Find\nCtrl/Cmd+G  Go to line");
                ui.add_space(14.0);
                ui.small("Runtime output opens source locations on click. Replay steps simulation frames; Lua breakpoints are not implemented.");
            });
            self.preferences.inspector_width = panel.response.rect.width();
            egui::CentralPanel::default().show(ctx, |ui| {
                if !enabled {
                    ui.disable();
                }
                egui::ScrollArea::horizontal()
                    .id_source("tabs")
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui
                                .selectable_label(self.show_overview, "Overview")
                                .clicked()
                            {
                                self.show_overview = true;
                            }
                            ui.separator();
                            for (index, tab) in workspace.tabs.iter().enumerate() {
                                let name =
                                    tab.path().file_name().unwrap_or_default().to_string_lossy();
                                let label =
                                    format!("{name}{}", if tab.dirty() { " *" } else { "" });
                                if ui
                                    .selectable_label(
                                        !self.show_overview && workspace.active == index,
                                        label,
                                    )
                                    .on_hover_text(tab.path().display().to_string())
                                    .clicked()
                                {
                                    workspace.active = index;
                                    self.show_overview = false;
                                }
                                if ui.small_button("x").on_hover_text("Close tab").clicked() {
                                    action = Some(Action::CloseTab(index));
                                }
                                ui.separator();
                            }
                        });
                    });
                ui.separator();
                if self.show_overview {
                    if let Some(next) =
                        crate::features::overview(ui, &workspace.config, &feature_status)
                    {
                        action = Some(next);
                    }
                    return;
                }
                if let Some(tab) = workspace.tabs.get_mut(workspace.active) {
                    ui.horizontal(|ui| {
                        ui.small(tab.path().display().to_string());
                        if let Tab::Code(doc) = tab {
                            let (line, column) = doc.line_column();
                            ui.small(format!("Ln {line}, Col {column}"));
                        }
                    });
                    if let Err(problem) =
                        tab.ui(ui, &mut self.code_tools, self.preferences.font_size)
                    {
                        error = Some(problem);
                    }
                } else {
                    ui.add_space(50.0);
                    ui.heading("Open a file to begin");
                    ui.label("Double-click a project file, or create a Lua file or sprite.");
                    if ui.button("Open feature overview").clicked() {
                        self.show_overview = true;
                    }
                }
            });
        }
        if let Some(browser_action) = browser_action {
            if let Err(problem) = self.browser_action(browser_action) {
                error = Some(problem);
            }
        }
        if let Some(error) = error {
            self.report(error);
        }
        action
    }
}

impl eframe::App for KairoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_process();
        for line in self.link.logs() {
            self.console.push(line);
        }
        let mut action = self.shortcuts(ctx);
        if let Some(next) = self.toolbar(ctx) {
            action = Some(next);
        }
        if let Some(next) = self.feature_bar(ctx) {
            action = Some(next);
        }
        if self.workspace.is_some() {
            egui::TopBottomPanel::bottom("workflow-status").show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.small(self.preferences.profile.name());
                    ui.separator();
                    ui.small(if self.running_game() {
                        "Local runtime running"
                    } else {
                        "Local runtime stopped"
                    });
                    if self.running_game() {
                        if let Some(sample) = &self.telemetry {
                            ui.small(format!(
                                "{:.0} FPS | Scene: {}",
                                sample.profile.fps, sample.inspection.scenes.current
                            ));
                        }
                    }
                    ui.separator();
                    ui.small(self.link.status());
                    if self.develop.dirty() {
                        ui.small("Unsaved tool changes");
                    }
                });
            });
            if let Some(next) = self.workspace_ui(ctx) {
                action = Some(next);
            }
        } else {
            let enabled = self.dialog.is_none();
            let mut hub_action = None;
            egui::CentralPanel::default().show(ctx, |ui| {
                if !enabled {
                    ui.disable();
                }
                hub_action = self.hub.ui(ui, &self.preferences.recent);
            });
            match hub_action {
                Some(HubAction::OpenDialog) => action = Some(Action::OpenDialog),
                Some(HubAction::Open(path)) => action = Some(Action::Open(path)),
                Some(HubAction::Remove(path)) => {
                    self.preferences.recent.retain(|item| *item != path)
                }
                Some(HubAction::Create {
                    path,
                    title,
                    template,
                }) => match kairo_project::create_project(&path, &title, template) {
                    Ok(_) => action = Some(Action::Open(path)),
                    Err(error) => self.report(error),
                },
                None => {}
            }
        }
        if ctx.input(|input| input.viewport().close_requested()) && !self.exit_allowed {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            action = Some(Action::Quit);
        }
        if let Some(action) = action {
            self.perform(action, ctx);
        }
        if self.dialog.is_none() {
            self.debug_panels(ctx);
            if self.show_link && self.dialog.is_none() {
                let root = self
                    .workspace
                    .as_ref()
                    .map(|workspace| workspace.files.root());
                let tester = self
                    .run_link
                    .as_ref()
                    .filter(|_| self.running_game())
                    .map(|connection| connection.address.as_str());
                if let Some(action) = self.link.ui(
                    ctx,
                    &mut self.show_link,
                    root,
                    self.process.is_some(),
                    tester,
                ) {
                    self.perform(action, ctx);
                }
            }
        }
        if self.dialog.is_none() {
            if self.develop.open {
                let sample = if self.running_game() {
                    self.telemetry.clone()
                } else {
                    None
                };
                let peers = self.link.peers();
                let action = self
                    .workspace
                    .as_mut()
                    .and_then(|workspace| self.develop.ui(ctx, workspace, sample.as_ref(), &peers));
                if let Some(action) = action {
                    self.perform(action, ctx);
                }
            }
            if let Some(action) = self.palette.ui(ctx) {
                self.perform(action, ctx);
            }
        }
        self.dialog_ui(ctx);
        self.api_ui(ctx);
        if self.last_refresh.elapsed() >= Duration::from_secs(2) {
            if let Some(workspace) = &mut self.workspace {
                if let Ok(nodes) = workspace.files.list() {
                    workspace.nodes = nodes;
                }
            }
            self.refresh_runtime_status();
            self.last_refresh = Instant::now();
        }
        ctx.request_repaint_after(if self.process.is_some() || self.link.hosting() {
            Duration::from_millis(40)
        } else {
            Duration::from_secs(2)
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.capture_tabs();
        eframe::set_value(storage, "preferences", &self.preferences);
    }
}
