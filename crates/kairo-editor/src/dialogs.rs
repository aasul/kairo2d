use crate::app::{Action, KairoApp};
use crate::feature_settings::{self, SettingsPage};
use crate::pixels::Pixels;
use crate::sprite::SpriteEditor;
use crate::tabs::Tab;
use anyhow::{ensure, Result};
use eframe::egui;
use kairo_project::SettingsDocument;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::OnceLock;

pub(crate) enum FileOperation {
    NewFile,
    NewFolder,
    Rename { from: PathBuf, preview: Vec<String> },
    Duplicate(PathBuf),
    SaveCopy(usize),
}

pub(crate) enum Dialog {
    File {
        operation: FileOperation,
        value: String,
    },
    Delete(PathBuf),
    Unsaved(Action),
    Sprite {
        path: String,
        width: u32,
        height: u32,
    },
    Settings {
        settings: Box<SettingsDocument>,
        page: SettingsPage,
    },
    UiGuide,
    Export {
        output: String,
    },
    Preferences,
    QuickOpen(String),
}

impl KairoApp {
    pub(crate) fn dialog_ui(&mut self, ctx: &egui::Context) {
        if let Some(mut dialog) = self.dialog.take() {
            let mut open = true;
            let mut done = false;
            let mut result: Result<()> = Ok(());
            let mut next = None;
            let title = match &dialog {
                Dialog::QuickOpen(_) => "Quick open",
                Dialog::File { .. } => "Project file",
                Dialog::Delete(_) => "Delete from project?",
                Dialog::Unsaved(_) => "Unsaved changes",
                Dialog::Sprite { .. } => "New sprite",
                Dialog::Settings { .. } => "Project settings",
                Dialog::UiGuide => "Lua UI - getting started",
                Dialog::Export { .. } => "Export game",
                Dialog::Preferences => "Preferences",
            };
            egui::Window::new(title).id(egui::Id::new("project-dialog")).open(&mut open)
                .collapsible(false).resizable(true).default_width(530.0).vscroll(true).show(ctx, |ui| {
                match &mut dialog {
                    Dialog::QuickOpen(query) => {
                        ui.add(egui::TextEdit::singleline(query).hint_text("File name or path").desired_width(420.0));
                        let mut selected = None;
                        if let Some(workspace) = &self.workspace {
                            let mut ranked:Vec<_>=workspace.nodes.iter().filter(|node|!node.directory).filter_map(|node| {
                                kairo_project::search::fuzzy_score(query,&node.relative.to_string_lossy()).map(|score| {
                                    let recent=workspace.recent_files.iter().position(|path|*path==node.relative).map_or(0,|n|8i64.saturating_sub(n as i64));
                                    (score+recent,node)
                                })
                            }).collect();
                            ranked.sort_by(|a,b|b.0.cmp(&a.0).then(a.1.relative.cmp(&b.1.relative)));
                            egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                                for (_,node) in ranked.into_iter().take(100) {
                                    if ui.selectable_label(false, node.relative.display().to_string()).clicked() { selected = Some(node.relative.clone()); }
                                }
                            });
                        }
                        if let (Some(path), Some(workspace)) = (selected, &mut self.workspace) {
                            result = workspace.open_file(path); done = result.is_ok();
                        }
                    }
                    Dialog::File { operation, value } => {
                        ui.label("Project-relative path");
                        ui.add(egui::TextEdit::singleline(value).desired_width(400.0));
                        ui.small("Existing destinations are never overwritten.");
                        if let FileOperation::Rename { preview, .. } = operation {
                            ui.small(format!("{} known reference(s) will be updated:", preview.len()));
                            for item in preview.iter().take(20) { ui.monospace(item); }
                            if preview.len() > 20 { ui.small("Additional references omitted from preview."); }
                            ui.small("Lua and relative tilemap references cannot be rewritten automatically; the move is refused when they may break.");
                        }
                        if ui.button("Apply").clicked() {
                            result = (|| {
                                let workspace = self.workspace.as_mut().ok_or_else(|| anyhow::anyhow!("no project is open"))?;
                                let path = PathBuf::from(value.replace('\\', "/"));
                                match operation {
                                    FileOperation::NewFile => {
                                        workspace.files.create_file(&path, b"local M = {}\n\nreturn M\n")?;
                                        workspace.open_file(path)?;
                                    }
                                    FileOperation::NewFolder => workspace.files.create_directory(path)?,
                                    FileOperation::Rename { from, .. } => workspace.rename(from, &path)?,
                                    FileOperation::Duplicate(from) => {
                                        ensure!(!workspace.tabs.iter().any(|tab| tab.path() == from.as_path() && tab.dirty()), "save this file before duplicating it");
                                        workspace.files.duplicate(from, path)?;
                                    }
                                    FileOperation::SaveCopy(index) => {
                                        let tab = workspace.tabs.get(*index).ok_or_else(|| anyhow::anyhow!("tab is no longer open"))?;
                                        let bytes = match tab {
                                            Tab::Code(doc) => doc.text.as_bytes().to_vec(),
                                            Tab::Animation(editor) => editor.encode()?,
                                            Tab::Sprite(sprite) => {
                                                ensure!(path.extension().is_some_and(|e| e == "png"), "sprite copies must use a .png extension");
                                                sprite.pixels.encode_png()?
                                            }
                                            _ => workspace.files.read(tab.path())?,
                                        };
                                        workspace.files.create_file(&path, &bytes)?;
                                        workspace.open_file(path)?;
                                    }
                                }
                                workspace.refresh()?;
                                Ok(())
                            })();
                            done = result.is_ok();
                        }
                    }
                    Dialog::Delete(path) => {
                        ui.label(path.display().to_string());
                        ui.label("This permanently deletes the selected file or folder from disk.");
                        ui.small("Save or close modified tabs first. Deleting this file cannot be undone.");
                        if ui.button("Delete permanently").clicked() {
                            result = self.workspace.as_mut().ok_or_else(|| anyhow::anyhow!("no project is open")).and_then(|workspace| workspace.delete(path));
                            done = result.is_ok();
                        }
                    }
                    Dialog::Unsaved(action) => {
                        ui.label("Save modified code, sprites, input mappings and particle presets before continuing?");
                        ui.horizontal(|ui| {
                            if ui.button("Save all and continue").clicked() {
                                result = self.workspace.as_mut().map_or(Ok(()), |workspace| {self.develop.save(workspace)?; workspace.save_all()});
                                if result.is_ok() { next = Some(action.clone()); done = true; }
                            }
                            if ui.button("Discard and continue").clicked() { next = Some(action.clone()); done = true; }
                            if ui.button("Cancel").clicked() { done = true; }
                        });
                    }
                    Dialog::Sprite { path, width, height } => {
                        ui.label("PNG path inside the project");
                        ui.text_edit_singleline(path);
                        ui.horizontal(|ui| {
                            ui.label("Width"); ui.add(egui::DragValue::new(width).range(1..=1024));
                            ui.label("Height"); ui.add(egui::DragValue::new(height).range(1..=1024));
                        });
                        ui.small("Transparent RGBA image. Nothing is written until you save.");
                        if ui.button("Create sprite").clicked() {
                            result = (|| {
                                let workspace = self.workspace.as_mut().ok_or_else(|| anyhow::anyhow!("no project is open"))?;
                                let path = PathBuf::from(path.replace('\\', "/"));
                                ensure!(path.extension().is_some_and(|e| e == "png"), "choose a .png filename");
                                ensure!(workspace.tabs.len() < 32, "close some tabs before creating another sprite");
                                ensure!(!workspace.files.resolve(&path)?.try_exists()?, "file already exists; open it from the browser instead");
                                ensure!(!workspace.tabs.iter().any(|tab| tab.path() == path), "this sprite is already open");
                                if !workspace.files.filesystem().exists("assets")? { workspace.files.create_directory("assets")?; }
                                workspace.tabs.push(Tab::Sprite(Box::new(SpriteEditor::new(Pixels::new(path, *width, *height)?))));
                                workspace.active = workspace.tabs.len() - 1;
                                Ok(())
                            })();
                            done = result.is_ok();
                        }
                    }
                    Dialog::Settings { settings, page } => {
                        feature_settings::ui(ui, settings, page);
                        ui.separator();
                        ui.small("Changes are saved to kairo.toml. Unknown keys and comments are preserved.");
                        ui.small("Running games, including Link testers, keep their previous settings until restarted.");
                        let running = self.running_game();
                        let can_run = !self.operation_active() || running;
                        let mut apply = None;
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Save settings").clicked() { apply = Some(false); }
                            if ui.add_enabled(can_run, egui::Button::new(if running { "Save & Restart" } else { "Save & Run" })).clicked() {
                                apply = Some(true);
                            }
                            if ui.button("Cancel").clicked() { done = true; }
                        });
                        if let Some(run) = apply {
                            result = self.save_project_settings(settings);
                            if result.is_ok() {
                                done = true;
                                if run { next = Some(if running { Action::Restart } else { Action::Run }); }
                            }
                        }
                    }
                    Dialog::UiGuide => {
                        ui.heading("Game UI");
                        ui.label("Use the Lua UI API to build the menus and HUDs that ship with your game. Development controls belong in game.debugUI.");
                        ui.code("hud = ui.panel({ padding = 12 })\nlabel = hud:add(ui.label(\"Score: 0\"))");
                        ui.label("The UI Workshop starter includes the complete load / update / draw wiring.");
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Game UI snippets").clicked() { next = Some(Action::ApiQuery("ui.".into())); done = true; }
                            if ui.button("New UI Workshop project").clicked() { next = Some(Action::NewProject(kairo_project::Template::Ui)); done = true; }
                        });
                        ui.separator();
                        ui.heading("Debug controls");
                        ui.label("Use debugui inside game.debugUI for egui-backed sliders, text and checkboxes. Run the game to interact with them.");
                        ui.code("function game.debugUI()\n    debugui.window(\"Player\", function()\n        debugui.text(\"Debug controls\")\n    end)\nend");
                        if ui.button("Debug UI snippets").clicked() { next = Some(Action::ApiQuery("debugui.".into())); done = true; }
                        ui.small("This guide includes Lua examples and snippets to help you get started.");
                        if ui.button("Close").clicked() { done = true; }
                    }
                    Dialog::Export { output } => {
                        ui.label(format!("Target: {} / {} (this host)", std::env::consts::OS, std::env::consts::ARCH));
                        ui.label("New output directory");
                        ui.add(egui::TextEdit::singleline(output).desired_width(420.0));
                        if ui.button("Choose parent...").clicked() {
                            if let Some(parent) = rfd::FileDialog::new().pick_folder() { *output = parent.join("kairo-game").display().to_string(); }
                        }
                        ui.small("Kairo checks the project and copies in a runtime built for this computer. Choose a new output folder for each export.");
                        if ui.button("Build package").clicked() { next = Some(Action::Build(PathBuf::from(&*output))); done = true; }
                    }
                    Dialog::Preferences => {
                        ui.checkbox(&mut self.preferences.overview_on_open, "Show the feature overview when opening a project");
                        ui.add(egui::Slider::new(&mut self.preferences.font_size, 10.0..=28.0).text("Code font size"));
                        ui.separator();
                        ui.label("Runtime executable");
                        ui.small(self.preferences.runtime.as_ref().map_or_else(|| "Automatic: bin/kairo or a development sibling binary".into(), |path| path.display().to_string()));
                        if ui.button("Choose runtime...").clicked() {
                            if let Some(path) = rfd::FileDialog::new().set_title("Choose kairo runtime, not Kairo editor").pick_file() { self.preferences.runtime = Some(path); self.refresh_runtime_status(); }
                        }
                        if ui.button("Use automatic detection").clicked() { self.preferences.runtime = None; self.refresh_runtime_status(); }
                        ui.small("Preferences, recent projects and window layout are stored in the platform user configuration directory, not the project.");
                        if ui.button("Done").clicked() { done = true; }
                    }
                }
            });
            if let Err(error) = result {
                self.report(error);
            }
            if done
                && matches!(
                    &dialog,
                    Dialog::QuickOpen(_) | Dialog::File { .. } | Dialog::Sprite { .. }
                )
            {
                self.focus_editor();
            }
            let confirmed = matches!(&dialog, Dialog::Unsaved(_));
            if open && !done {
                self.dialog = Some(dialog);
            }
            if let Some(action) = next {
                if let Err(error) = self.execute(action, ctx, confirmed) {
                    self.report(error);
                }
            }
        }
        if let Some(message) = self.error.clone() {
            let mut open = true;
            let mut done = false;
            egui::Window::new("Kairo error")
                .open(&mut open)
                .collapsible(false)
                .default_width(480.0)
                .show(ctx, |ui| {
                    ui.label(message);
                    if ui.button("Dismiss").clicked() {
                        done = true;
                    }
                });
            if !open || done {
                self.error = None;
            }
        }
    }

    pub(crate) fn api_ui(&mut self, ctx: &egui::Context) {
        if !self.show_api || self.dialog.is_some() {
            return;
        }
        static API: OnceLock<Result<Vec<ApiEntry>, String>> = OnceLock::new();
        let entries = match API.get_or_init(|| {
            serde_json::from_str(include_str!("../../../docs/lua-api.json"))
                .map_err(|error| error.to_string())
        }) {
            Ok(entries) => entries,
            Err(error) => {
                self.error = Some(error.clone());
                self.show_api = false;
                return;
            }
        };
        let mut open = self.show_api;
        let mut insert = None;
        egui::Window::new("Kairo Lua API").open(&mut open).default_width(560.0).default_height(360.0).show(ctx, |ui| {
            ui.add(egui::TextEdit::singleline(&mut self.api_query).hint_text("Filter module or function"));
            ui.small("Click a function signature to insert it into the active Lua tab, or copy it for later.");
            ui.separator();
            let query = self.api_query.to_lowercase();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for entry in entries.iter().filter(|entry| {
                    let signature = entry.signature.to_lowercase();
                    if query.ends_with('.') { signature.starts_with(&query) } else { signature.contains(&query) }
                }) {
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(&entry.signature).on_hover_text(&entry.description).clicked() { insert = Some(entry.snippet.clone()); }
                        if ui.small_button("Copy").clicked() { ui.output_mut(|output| output.copied_text = entry.snippet.clone()); }
                    });
                }
            });
        });
        self.show_api = open;
        if let Some(snippet) = insert {
            if let Some(workspace) = &mut self.workspace {
                if let Some(Tab::Code(doc)) = workspace.tabs.get_mut(workspace.active) {
                    doc.replace_selection(&snippet);
                    self.show_api = false;
                    self.focus_editor();
                }
            }
        }
    }
}

#[derive(Deserialize)]
struct ApiEntry {
    signature: String,
    snippet: String,
    description: String,
}
