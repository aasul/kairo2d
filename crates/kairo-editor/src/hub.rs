use eframe::egui::{self, RichText};
use kairo_project::Template;
use std::path::PathBuf;

pub enum HubAction {
    Create {
        path: PathBuf,
        title: String,
        template: Template,
    },
    OpenDialog,
    Open(PathBuf),
    Remove(PathBuf),
}

pub struct Hub {
    title: String,
    folder: String,
    parent: PathBuf,
    template: Template,
}

impl Default for Hub {
    fn default() -> Self {
        Self {
            title: "My Game".into(),
            folder: "my-game".into(),
            parent: std::env::current_dir().unwrap_or_default(),
            template: Template::HelloWorld,
        }
    }
}

fn template_label(template: Template) -> &'static str {
    match template {
        Template::Micro => "Fantasy Console (Micro)",
        _ => template.name(),
    }
}

fn template_description(template: Template) -> &'static str {
    match template {
        Template::Scenes => "A menu, a play scene, and a pause screen.",
        Template::InputActions => "Set up keyboard, mouse, and controller bindings in input.toml.",
        Template::Inspector => "Expose game values and try editing them while the game runs.",
        Template::Particles => "Move the emitter with the mouse, then edit and save its preset.",
        Template::Mixer => "Try the four audio buses, with volume, panning, and mute controls.",
        Template::ReplayBugs => "Save a bug bookmark, inspect it, and restore the captured state.",
        Template::TopDown => {
            "Signal Yard uses scenes, input actions, tile collisions, animation, sound, and saves."
        }
        Template::Platformer => {
            "A small platform game with jumping, particles, and inspector controls."
        }
        Template::Empty => "An empty Lua lifecycle. Add your own game code.",
        Template::HelloWorld => "Shapes and text with a small, readable starter script.",
        Template::Movement => "Move a sprite with keyboard controls.",
        Template::Breakout => "An arcade game with sound, collisions and restart logic.",
        Template::Micro => "A small playable game set up for a 320 x 180 canvas.",
        Template::Ui => "A game HUD and a few interactive development controls, all wired in Lua.",
        Template::Link => "A live-testing starter with save and restore hooks for game state.",
        Template::Replay => {
            "Registered game state with recording enabled. Open Replay to browse snapshots."
        }
    }
}

impl Hub {
    pub(crate) fn select_template(&mut self, template: Template) {
        self.template = template;
        let (title, folder) = match template {
            Template::Micro => ("My Micro Game", "micro-game"),
            Template::Link => ("Link Demo", "link-demo"),
            Template::Replay => ("Replay Demo", "replay-demo"),
            Template::Ui => ("UI Workshop", "ui-workshop"),
            _ => ("My Game", "my-game"),
        };
        self.title = title.into();
        self.folder = folder.into();
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, recent: &[PathBuf]) -> Option<HubAction> {
        let mut action = None;
        egui::ScrollArea::vertical().id_source("project-hub").show(ui, |ui| {
            ui.add_space(16.0);
            ui.label(RichText::new("Kairo").size(28.0).strong());
            ui.label("Make a game. Start from a focused demo or the playable Signal Yard starter.");
            ui.small(format!("Version {} | Workflow Tools", env!("CARGO_PKG_VERSION")));
            ui.add_space(20.0);
            ui.columns(2, |columns| {
                let ui = &mut columns[0];
                ui.heading("Create a project");
                ui.label("Choose a starter");
                for row in Template::ALL.chunks(2) {
                    ui.columns(2, |columns| {
                        for (column, template) in columns.iter_mut().zip(row) {
                            let label = template_label(*template);
                            let response = column.add_sized(
                                [column.available_width(), 34.0],
                                egui::SelectableLabel::new(self.template == *template, label),
                            );
                            if response.on_hover_text(template_description(*template)).clicked() { self.template = *template; }
                        }
                    });
                }
                ui.group(|ui| { ui.label(template_description(self.template)); });
                ui.add_space(8.0);
                ui.label("Project title");
                ui.text_edit_singleline(&mut self.title);
                ui.label("Folder name");
                ui.text_edit_singleline(&mut self.folder);
                ui.label("Location");
                ui.horizontal_wrapped(|ui| {
                    ui.label(self.parent.display().to_string());
                    if ui.button("Choose...").clicked() {
                        if let Some(parent) = rfd::FileDialog::new().set_title("Choose a parent folder").pick_folder() { self.parent = parent; }
                    }
                });
                ui.add_space(10.0);
                let valid_folder = !self.folder.trim().is_empty() && self.folder != "." && self.folder != ".."
                    && !self.folder.contains(['/', '\\', ':']) && !self.title.trim().is_empty();
                if ui.add_enabled(valid_folder, egui::Button::new(format!("Create {} project", template_label(self.template)))).clicked() {
                    action = Some(HubAction::Create { path: self.parent.join(&self.folder), title: self.title.clone(), template: self.template });
                }
                ui.small("Kairo creates this project in a new folder. Your other projects stay as they are.");

                let ui = &mut columns[1];
                ui.horizontal_wrapped(|ui| {
                    ui.heading("Recent projects");
                    if ui.button("Open folder...").clicked() { action = Some(HubAction::OpenDialog); }
                });
                ui.add_space(12.0);
                if recent.is_empty() { ui.label("No recent projects yet."); }
                for path in recent {
                    ui.horizontal_wrapped(|ui| {
                        let name = path.file_name().unwrap_or_default().to_string_lossy();
                        if ui.button(name.as_ref()).on_hover_text(path.display().to_string()).clicked() { action = Some(HubAction::Open(path.clone())); }
                        if ui.small_button("Remove").on_hover_text("Remove from this list, not from disk").clicked() { action = Some(HubAction::Remove(path.clone())); }
                    });
                    ui.small(path.display().to_string());
                    ui.add_space(10.0);
                }
                ui.add_space(12.0);
                ui.separator();
                ui.strong("Already have a project?");
                ui.label("Open it, then use the FEATURES bar: Fantasy Console, Link, Replay, Profiler and Lua UI.");
                ui.label("You can enable Fantasy Console for an existing project. Start Link when you are ready to invite a trusted tester. Replay captures the state you register.");
                ui.small("F6 Overview | F7 Fantasy Console | F8 Link | F9 Replay | F10 Profiler");
            });
            ui.add_space(18.0);
            ui.separator();
            ui.small("Project scripts start only when you choose Run. Open projects and Link sessions from people you trust.");
        });
        action
    }
}
