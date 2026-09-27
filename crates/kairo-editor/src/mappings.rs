use crate::workspace::Workspace;
use anyhow::Result;
use eframe::egui;
use kairo_core::actions::InputBindings;

#[derive(Default)]
pub struct MappingEditor {
    bindings: InputBindings,
    disk: Option<Vec<u8>>,
    saved: String,
    loaded: bool,
    name: String,
    status: String,
}
fn key_list(ui: &mut egui::Ui, label: &str, values: &mut Vec<String>) {
    let id = ui.make_persistent_id(label);
    let mut text = ui
        .data(|data| data.get_temp::<String>(id))
        .unwrap_or_else(|| values.join(", "));
    if !ui.memory(|memory| memory.has_focus(id)) {
        text = values.join(", ");
    }
    ui.label(label);
    if ui
        .add(egui::TextEdit::singleline(&mut text).id(id))
        .changed()
    {
        *values = text
            .split(',')
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_owned)
            .collect();
    }
    ui.data_mut(|data| data.insert_temp(id, text));
}
impl MappingEditor {
    pub fn dirty(&self) -> bool {
        self.loaded && toml::to_string(&self.bindings).is_ok_and(|v| v != self.saved)
    }
    pub fn load(&mut self, workspace: &Workspace) -> Result<()> {
        self.disk = if workspace.files.resolve("input.toml")?.try_exists()? {
            Some(workspace.files.read("input.toml")?)
        } else {
            None
        };
        self.bindings = match &self.disk {
            Some(v) => InputBindings::parse(std::str::from_utf8(v)?)?,
            None => InputBindings::default(),
        };
        self.saved = toml::to_string(&self.bindings)?;
        self.loaded = true;
        self.status = "Project input.toml loaded. Saved mappings take effect on restart.".into();
        Ok(())
    }
    pub fn save(&mut self, workspace: &mut Workspace) -> Result<()> {
        self.bindings.validate()?;
        let bytes = toml::to_string_pretty(&self.bindings)?.into_bytes();
        workspace.save_tool_file(
            std::path::Path::new("input.toml"),
            self.disk.as_deref(),
            &bytes,
        )?;
        self.disk = Some(bytes);
        self.saved = toml::to_string(&self.bindings)?;
        self.status = "Saved input.toml. Restart the game to apply.".into();
        Ok(())
    }
    pub fn ui(&mut self, ui: &mut egui::Ui, workspace: &mut Workspace) -> Result<()> {
        if !self.loaded {
            self.load(workspace)?;
        }
        ui.label("Named actions and axes | input.toml");
        ui.small("Comma-separated lowercase key/button names. Lua input.bind calls can override these defaults.");
        ui.horizontal(|ui| {
            if ui.button("Save mappings").clicked() {
                if let Err(e) = self.save(workspace) {
                    self.status = format!("{e:#}");
                }
            }
            if ui.button("Discard edits / reload").clicked() {
                if let Err(e) = self.load(workspace) {
                    self.status = format!("{e:#}");
                }
            }
            if self.dirty() {
                ui.label("Modified");
            }
        });
        ui.label(&self.status);
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.name).hint_text("New binding name"));
            if ui.button("+ Action").clicked()
                && !self.name.is_empty()
                && self.bindings.actions.len() < 128
            {
                self.bindings.actions.entry(self.name.clone()).or_default();
            }
            if ui.button("+ Axis").clicked()
                && !self.name.is_empty()
                && self.bindings.axes.len() < 64
            {
                self.bindings.axes.entry(self.name.clone()).or_default();
            }
        });
        let mut remove = None;
        for (name, action) in &mut self.bindings.actions {
            egui::CollapsingHeader::new(format!("Action: {name}"))
                .id_source(("action", name))
                .show(ui, |ui| {
                    key_list(ui, "Keyboard", &mut action.keyboard);
                    key_list(ui, "Gamepad buttons", &mut action.gamepad);
                    ui.horizontal(|ui| {
                        ui.label("Mouse");
                        for button in 1..=5 {
                            let mut on = action.mouse.contains(&button);
                            if ui.checkbox(&mut on, button.to_string()).changed() {
                                action.mouse.retain(|v| *v != button);
                                if on {
                                    action.mouse.push(button);
                                }
                            }
                        }
                    });
                    ui.add(
                        egui::DragValue::new(&mut action.player)
                            .range(1..=16)
                            .prefix("Controller ID "),
                    );
                    if ui.button("Remove action").clicked() {
                        remove = Some(name.clone());
                    }
                });
        }
        if let Some(name) = remove {
            self.bindings.actions.remove(&name);
        }
        let mut remove = None;
        for (name, axis) in &mut self.bindings.axes {
            egui::CollapsingHeader::new(format!("Axis: {name}"))
                .id_source(("axis", name))
                .show(ui, |ui| {
                    key_list(ui, "Negative keys", &mut axis.negative);
                    key_list(ui, "Positive keys", &mut axis.positive);
                    egui::ComboBox::from_id_source(("stick", name))
                        .selected_text(axis.gamepad_axis.as_deref().unwrap_or("None"))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut axis.gamepad_axis, None, "None");
                            for value in [
                                "left_x",
                                "left_y",
                                "right_x",
                                "right_y",
                                "left_trigger",
                                "right_trigger",
                            ] {
                                ui.selectable_value(
                                    &mut axis.gamepad_axis,
                                    Some(value.into()),
                                    value,
                                );
                            }
                        });
                    ui.add(egui::Slider::new(&mut axis.dead_zone, 0.0..=0.95).text("Dead zone"));
                    ui.add(
                        egui::DragValue::new(&mut axis.player)
                            .range(1..=16)
                            .prefix("Controller ID "),
                    );
                    if ui.button("Remove axis").clicked() {
                        remove = Some(name.clone());
                    }
                });
        }
        if let Some(name) = remove {
            self.bindings.axes.remove(&name);
        }
        Ok(())
    }
}
