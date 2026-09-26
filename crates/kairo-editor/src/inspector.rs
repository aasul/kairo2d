use eframe::egui;
use kairo_core::inspector::{InspectNode, InspectUpdate};
use kairo_core::profiler::{DebugCommand, SceneOperation, Telemetry};
use serde_json::Value;
use std::collections::BTreeMap;

struct Draft {
    expected: Value,
    value: Value,
    dirty: bool,
}
#[derive(Default)]
pub struct InspectorPanel {
    identity: (u64, u64),
    drafts: BTreeMap<String, Draft>,
    scene: String,
    language: String,
    bookmarks: crate::bookmark_panel::BookmarkPanel,
}
fn value_ui(ui: &mut egui::Ui, value: &mut Value, node: &InspectNode) -> bool {
    match value {
        Value::Bool(value) => ui.checkbox(value, "").changed(),
        Value::Number(value) => {
            let mut number = value.as_f64().unwrap_or(0.0);
            let changed = ui
                .add(egui::DragValue::new(&mut number).speed(0.1).range(
                    node.metadata.min.unwrap_or(-1.0e12)..=node.metadata.max.unwrap_or(1.0e12),
                ))
                .changed();
            if changed {
                if let Some(next) = serde_json::Number::from_f64(number) {
                    *value = next;
                }
            }
            changed
        }
        Value::String(value) => {
            if node.metadata.choices.is_empty() {
                ui.add(egui::TextEdit::singleline(value).char_limit(512))
                    .changed()
            } else {
                let before = value.clone();
                egui::ComboBox::from_id_source("enum-value")
                    .selected_text(value.as_str())
                    .show_ui(ui, |ui| {
                        for choice in &node.metadata.choices {
                            ui.selectable_value(value, choice.clone(), choice);
                        }
                    });
                *value != before
            }
        }
        Value::Array(values) => {
            let mut changed = false;
            for (index, value) in values.iter_mut().enumerate() {
                ui.push_id(index, |ui| {
                    ui.horizontal(|ui| {
                        ui.small(index.to_string());
                        changed |= value_ui(ui, value, node);
                    });
                });
            }
            changed
        }
        Value::Object(values) => {
            let mut changed = false;
            for (name, value) in values {
                ui.push_id(name, |ui| {
                    ui.horizontal(|ui| {
                        ui.small(name);
                        changed |= value_ui(ui, value, node);
                    });
                });
            }
            changed
        }
        Value::Null => {
            ui.label("Unsupported null");
            false
        }
    }
}
impl InspectorPanel {
    pub fn ui(&mut self, ui: &mut egui::Ui, peer: u64, sample: &Telemetry) -> Option<DebugCommand> {
        let snapshot = &sample.inspection;
        if self.identity != (peer, snapshot.session) {
            self.identity = (peer, snapshot.session);
            self.drafts.clear();
            self.scene.clear();
        }
        let mut command = None;
        if let Some(error) = &snapshot.error {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
        ui.label(format!(
            "Runtime session {} | {} exposed fields",
            snapshot.session,
            snapshot.nodes.len()
        ));
        ui.small("Only explicitly exposed fields are shown. Edit, then Apply. Stale values are rejected; pause or Reset to refresh.");
        ui.horizontal(|ui| {
            if ui
                .button(if sample.replay.paused {
                    "Resume"
                } else {
                    "Pause"
                })
                .clicked()
            {
                command = Some(if sample.replay.paused {
                    DebugCommand::Resume
                } else {
                    DebugCommand::Pause
                });
            }
            if ui.button("Single step").clicked() {
                command = Some(DebugCommand::Step);
            }
            ui.label(format!(
                "{:.1} FPS | {:.2}s",
                sample.profile.fps, sample.replay.time
            ));
        });
        self.drafts
            .retain(|path, _| snapshot.nodes.iter().any(|n| n.path == *path));
        let mut groups: BTreeMap<&str, Vec<&InspectNode>> = BTreeMap::new();
        for node in &snapshot.nodes {
            groups
                .entry(node.path.split('.').next().unwrap_or("Runtime"))
                .or_default()
                .push(node);
        }
        for (group, nodes) in groups {
            egui::CollapsingHeader::new(group)
                .default_open(true)
                .show(ui, |ui| {
                    for node in nodes {
                        let draft = self
                            .drafts
                            .entry(node.path.clone())
                            .or_insert_with(|| Draft {
                                expected: node.value.clone(),
                                value: node.value.clone(),
                                dirty: false,
                            });
                        if !draft.dirty {
                            draft.expected = node.value.clone();
                            draft.value = node.value.clone();
                        }
                        ui.push_id(&node.path, |ui| {
                            ui.group(|ui| {
                                ui.strong(&node.path);
                                if node.metadata.writable {
                                    if value_ui(ui, &mut draft.value, node) {
                                        draft.dirty = true;
                                    }
                                    ui.horizontal(|ui| {
                                        if ui
                                            .add_enabled(draft.dirty, egui::Button::new("Apply"))
                                            .clicked()
                                        {
                                            command = Some(DebugCommand::Inspect {
                                                update: Box::new(InspectUpdate {
                                                    session: snapshot.session,
                                                    path: node.path.clone(),
                                                    expected: draft.expected.clone(),
                                                    value: draft.value.clone(),
                                                }),
                                            });
                                            // Wait for telemetry to confirm the write, rather than optimistically changing the runtime value.
                                        }
                                        if ui.button("Reset").clicked() {
                                            draft.dirty = false;
                                        }
                                        if draft.dirty && node.value == draft.value {
                                            draft.dirty = false;
                                        } else if draft.dirty && node.value != draft.expected {
                                            ui.colored_label(
                                                egui::Color32::YELLOW,
                                                "Changed remotely: Reset before applying",
                                            );
                                        }
                                    });
                                } else {
                                    ui.monospace(node.value.to_string());
                                    ui.small("Read-only");
                                }
                            });
                        });
                    }
                });
        }
        if snapshot.nodes.is_empty() {
            ui.code("inspector.expose(\"player.speed\", player, \"speed\", {writable=true, min=0, max=500})");
        }
        ui.separator();
        ui.strong("Scenes");
        ui.label(format!("Stack: {}", snapshot.scenes.stack.join(" > ")));
        if self.scene.is_empty() {
            self.scene = snapshot.scenes.current.clone();
        }
        egui::ComboBox::from_id_source("scene-choice")
            .selected_text(&self.scene)
            .show_ui(ui, |ui| {
                for name in &snapshot.scenes.registered {
                    ui.selectable_value(&mut self.scene, name.clone(), name);
                }
            });
        ui.horizontal(|ui| {
            for (label, operation, enabled) in [
                ("Switch", SceneOperation::Switch, !self.scene.is_empty()),
                ("Push", SceneOperation::Push, !self.scene.is_empty()),
                ("Pop", SceneOperation::Pop, snapshot.scenes.stack.len() > 1),
                (
                    "Reload",
                    SceneOperation::Reload,
                    !snapshot.scenes.stack.is_empty(),
                ),
            ] {
                if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
                    command = Some(DebugCommand::Scene {
                        session: snapshot.session,
                        operation,
                        name: Some(self.scene.clone()),
                    });
                }
            }
        });
        ui.separator();
        ui.strong("Debug drawing");
        let mut flags = sample.debug_draw;
        let mut changed = ui
            .checkbox(&mut flags.physics, "Collider outlines and body centers")
            .changed();
        changed |= ui
            .checkbox(&mut flags.velocities, "Velocity vectors (with physics)")
            .changed();
        changed |= ui
            .checkbox(&mut flags.bounds, "Textured sprite bounds")
            .changed();
        if changed {
            command = Some(DebugCommand::DebugDraw {
                physics: flags.physics,
                bounds: flags.bounds,
                velocities: flags.velocities,
            });
        }
        ui.separator();
        ui.strong("Game localization preview");
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.language).hint_text("en, fr, de..."));
            if ui.button("Set game language").clicked() {
                command = Some(DebugCommand::Language {
                    language: self.language.clone(),
                });
            }
        });
        ui.separator();
        if let Some(next) = self.bookmarks.ui(ui, peer, sample) {
            command = Some(next);
        }
        command
    }
}

pub fn mixer_ui(ui: &mut egui::Ui, sample: &Telemetry) -> Option<DebugCommand> {
    let mut command = None;
    ui.label("Live audio buses | changes affect the selected runtime, not project defaults");
    for bus in &sample.mixer {
        ui.push_id(&bus.name, |ui| {
            ui.horizontal(|ui| {
                ui.strong(&bus.name);
                ui.small(format!("{} voices", bus.voices));
                let mut volume = bus.volume;
                let mut muted = bus.muted;
                let mut changed = ui
                    .add(egui::Slider::new(&mut volume, 0.0..=1.0).text("Volume"))
                    .changed();
                changed |= ui.checkbox(&mut muted, "Mute").changed();
                let stop = ui.button("Stop voices").clicked();
                if changed || stop {
                    command = Some(DebugCommand::Mixer {
                        bus: bus.name.clone(),
                        volume: Some(volume),
                        muted: Some(muted),
                        fade: 0.1,
                        stop,
                    });
                }
            });
        });
    }
    ui.small("Master affects every bus. Fades use 100 ms here. A missing audio device keeps controls usable but silent.");
    command
}
