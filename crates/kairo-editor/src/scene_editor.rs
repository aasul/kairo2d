//! Focused scene-resource editor. The hierarchy is the same validated resource
//! loaded by the runtime; editing does not create a parallel editor scene model.
use anyhow::{ensure, Context, Result};
use eframe::egui::{self, Color32, Key, Modifiers, Pos2, Sense, Stroke, Vec2};
use kairo_core::scene_graph::{NodeFile, NodeId, NodeKind, SceneGraph, Transform2D};
use kairo_project::ProjectFiles;
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;

const HISTORY_BYTES: usize = 32 * 1024 * 1024;

struct SceneEdit {
    label: String,
    before: String,
    after: String,
}

#[derive(Clone)]
struct NativeDraft {
    path: String,
    text: String,
    shape: String,
    bus: String,
    width: f32,
    height: f32,
    radius: f32,
    zoom: f32,
    scale: f32,
    volume: f32,
    color: [f32; 4],
    active: bool,
    interactive: bool,
    autoplay: bool,
    looping: bool,
}

impl Default for NativeDraft {
    fn default() -> Self {
        Self {
            path: String::new(),
            text: String::new(),
            shape: "rectangle".into(),
            bus: "sfx".into(),
            width: 32.0,
            height: 32.0,
            radius: 16.0,
            zoom: 1.0,
            scale: 2.0,
            volume: 1.0,
            color: [1.0; 4],
            active: false,
            interactive: false,
            autoplay: false,
            looping: false,
        }
    }
}

impl NativeDraft {
    fn from_node(node: &NodeFile) -> Self {
        let mut draft = Self::default();
        let values = &node.properties;
        let number = |key: &str, default: f32| {
            values
                .get(key)
                .and_then(serde_json::Value::as_f64)
                .map_or(default, |value| value as f32)
        };
        draft.path = values
            .get(if node.kind == NodeKind::AudioSource {
                "sound"
            } else {
                "texture"
            })
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_owned();
        draft.text = values
            .get("text")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_owned();
        draft.shape = values
            .get("shape")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("rectangle")
            .to_owned();
        draft.bus = values
            .get("bus")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("sfx")
            .to_owned();
        draft.width = number("width", draft.width);
        draft.height = number("height", draft.height);
        draft.radius = number("radius", draft.radius);
        draft.zoom = number("zoom", draft.zoom);
        draft.scale = number("scale", draft.scale);
        draft.volume = number("volume", draft.volume);
        draft.active = values
            .get("active")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        draft.interactive = values
            .get("interactive")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        draft.autoplay = values
            .get("autoplay")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        draft.looping = values
            .get("looping")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        if let Some(channels) = values
            .get(
                if matches!(node.kind, NodeKind::Sprite | NodeKind::AnimatedSprite) {
                    "tint"
                } else {
                    "color"
                },
            )
            .and_then(serde_json::Value::as_array)
        {
            for (index, channel) in channels.iter().take(4).enumerate() {
                if let Some(value) = channel.as_f64() {
                    draft.color[index] = value as f32;
                }
            }
        }
        draft
    }

    fn apply(&self, graph: &mut SceneGraph, id: NodeId, kind: NodeKind) -> Result<()> {
        let set = |graph: &mut SceneGraph, key: &str, value| graph.set_property(id, key, value);
        match kind {
            NodeKind::Sprite | NodeKind::AnimatedSprite => {
                ensure!(
                    self.width > 0.0 && self.height > 0.0,
                    "sprite size must be positive"
                );
                if self.path.is_empty() {
                    graph.remove_property(id, "texture")?;
                } else {
                    set(graph, "texture", serde_json::json!(self.path))?;
                }
                set(graph, "width", serde_json::json!(self.width))?;
                set(graph, "height", serde_json::json!(self.height))?;
                set(graph, "tint", serde_json::json!(self.color))?;
            }
            NodeKind::Camera2D => {
                ensure!(
                    self.zoom.is_finite() && self.zoom > 0.0,
                    "camera zoom must be positive"
                );
                set(graph, "active", serde_json::json!(self.active))?;
                set(graph, "zoom", serde_json::json!(self.zoom))?;
            }
            NodeKind::Collider2D => {
                ensure!(
                    self.shape == "rectangle" || self.shape == "circle",
                    "unknown collider shape"
                );
                set(graph, "shape", serde_json::json!(self.shape))?;
                if self.shape == "rectangle" {
                    ensure!(
                        self.width > 0.0 && self.height > 0.0,
                        "collider size must be positive"
                    );
                    set(graph, "width", serde_json::json!(self.width))?;
                    set(graph, "height", serde_json::json!(self.height))?;
                } else {
                    ensure!(self.radius > 0.0, "collider radius must be positive");
                    set(graph, "radius", serde_json::json!(self.radius))?;
                }
            }
            NodeKind::Control => {
                ensure!(
                    self.width >= 0.0 && self.height >= 0.0,
                    "control size cannot be negative"
                );
                set(graph, "width", serde_json::json!(self.width))?;
                set(graph, "height", serde_json::json!(self.height))?;
                set(graph, "color", serde_json::json!(self.color))?;
                set(graph, "interactive", serde_json::json!(self.interactive))?;
            }
            NodeKind::Text => {
                ensure!(
                    self.scale > 0.0 && self.text.len() <= 4096,
                    "invalid text or scale"
                );
                set(graph, "text", serde_json::json!(self.text))?;
                set(graph, "scale", serde_json::json!(self.scale))?;
                set(graph, "color", serde_json::json!(self.color))?;
            }
            NodeKind::AudioSource => {
                ensure!(
                    self.volume.is_finite() && (0.0..=1.0).contains(&self.volume),
                    "volume must be 0..=1"
                );
                ensure!(
                    !self.autoplay || !self.path.is_empty(),
                    "autoplay needs a sound path"
                );
                if self.path.is_empty() {
                    graph.remove_property(id, "sound")?;
                } else {
                    set(graph, "sound", serde_json::json!(self.path))?;
                }
                set(graph, "autoplay", serde_json::json!(self.autoplay))?;
                set(graph, "looping", serde_json::json!(self.looping))?;
                set(graph, "volume", serde_json::json!(self.volume))?;
                set(graph, "bus", serde_json::json!(self.bus))?;
            }
            _ => {}
        }
        ensure!(
            self.color
                .iter()
                .all(|channel| channel.is_finite() && (0.0..=1.0).contains(channel)),
            "color channels must be 0..=1"
        );
        Ok(())
    }
}

pub struct SceneEditor {
    pub path: PathBuf,
    graph: SceneGraph,
    disk: Vec<u8>,
    saved: String,
    dirty: bool,
    selected: NodeId,
    collapsed: HashSet<u64>,
    filter: String,
    name: String,
    parent_path: String,
    script: String,
    transform: Transform2D,
    native: NativeDraft,
    property_name: String,
    property_json: String,
    kind: usize,
    zoom: f32,
    pan: Vec2,
    snap: bool,
    dragging: Option<(NodeId, Pos2)>,
    preview_size: Vec2,
    undo: VecDeque<SceneEdit>,
    redo: VecDeque<SceneEdit>,
    error: Option<String>,
}

const KINDS: &[(&str, NodeKind)] = &[
    ("Node", NodeKind::Node),
    ("Node2D", NodeKind::Node2D),
    ("Sprite", NodeKind::Sprite),
    ("AnimatedSprite", NodeKind::AnimatedSprite),
    ("Camera2D", NodeKind::Camera2D),
    ("StaticBody2D", NodeKind::StaticBody2D),
    ("DynamicBody2D", NodeKind::DynamicBody2D),
    ("CharacterBody2D", NodeKind::CharacterBody2D),
    ("Collider2D", NodeKind::Collider2D),
    ("Area2D", NodeKind::Area2D),
    ("TileMap", NodeKind::TileMap),
    ("ParticleEmitter", NodeKind::ParticleEmitter),
    ("AudioSource", NodeKind::AudioSource),
    ("Text", NodeKind::Text),
    ("CanvasLayer", NodeKind::CanvasLayer),
    ("Control", NodeKind::Control),
    ("ScriptNode", NodeKind::ScriptNode),
];

impl SceneEditor {
    pub fn load(files: &ProjectFiles, path: PathBuf) -> Result<Self> {
        let config = kairo_core::Config::load(files.filesystem()).unwrap_or_default();
        let disk = files.read(&path)?;
        let graph = if disk.is_empty() {
            let name = path
                .file_stem()
                .and_then(|name| name.to_str())
                .context("scene filename must be UTF-8")?;
            SceneGraph::new(name)?
        } else {
            SceneGraph::from_json(std::str::from_utf8(&disk).context("scene is not UTF-8")?)?
        };
        let saved = graph.to_json()?;
        let selected = graph.root();
        let mut editor = Self {
            path,
            graph,
            disk,
            saved,
            dirty: false,
            selected,
            collapsed: HashSet::new(),
            filter: String::new(),
            name: String::new(),
            parent_path: String::new(),
            script: String::new(),
            transform: Transform2D::default(),
            native: NativeDraft::default(),
            property_name: String::new(),
            property_json: "null".into(),
            kind: 1,
            zoom: 1.0,
            pan: Vec2::ZERO,
            snap: false,
            dragging: None,
            preview_size: Vec2::new(config.game.width as f32, config.game.height as f32),
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            error: None,
        };
        editor.select(selected)?;
        Ok(editor)
    }

    pub fn dirty(&self) -> bool {
        self.dirty || self.disk.is_empty()
    }

    pub fn save(&mut self, files: &ProjectFiles) -> Result<()> {
        ensure!(
            files.read(&self.path)? == self.disk,
            "scene changed on disk; reload before saving"
        );
        let source = self.graph.to_json()?;
        files.write(&self.path, source.as_bytes())?;
        self.disk = source.as_bytes().to_vec();
        self.saved = source;
        self.dirty = false;
        Ok(())
    }

    fn select(&mut self, id: NodeId) -> Result<()> {
        let node = self.graph.node(id)?;
        self.selected = id;
        self.name = node.data().name.clone();
        self.parent_path = node
            .parent()
            .map(|parent| self.graph.path(parent))
            .transpose()?
            .unwrap_or_default();
        self.script = node.data().script.clone().unwrap_or_default();
        self.transform = node.data().transform;
        self.native = NativeDraft::from_node(node.data());
        Ok(())
    }

    fn edit<T>(&mut self, label: &str, f: impl FnOnce(&mut SceneGraph) -> Result<T>) -> Result<T> {
        let before = self.graph.to_json()?;
        let result = match f(&mut self.graph) {
            Ok(result) => result,
            Err(error) => {
                self.restore(&before)?;
                return Err(error);
            }
        };
        let after = match self.graph.to_json() {
            Ok(after) => after,
            Err(error) => {
                self.restore(&before)?;
                return Err(error);
            }
        };
        if before != after {
            self.undo.push_back(SceneEdit {
                label: label.into(),
                before,
                after: after.clone(),
            });
            self.redo.clear();
            while self
                .undo
                .iter()
                .map(|edit| edit.before.len() + edit.after.len())
                .sum::<usize>()
                > HISTORY_BYTES
                && self.undo.len() > 1
            {
                self.undo.pop_front();
            }
            self.dirty = after != self.saved;
        }
        Ok(result)
    }

    fn restore(&mut self, source: &str) -> Result<()> {
        let file_id = self
            .graph
            .node(self.selected)
            .ok()
            .map(|node| node.data().id);
        self.graph = SceneGraph::from_json(source)?;
        let selected = file_id
            .and_then(|id| self.graph.find_by_file_id(id))
            .unwrap_or(self.graph.root());
        self.select(selected)?;
        self.dirty = source != self.saved;
        Ok(())
    }

    pub fn undo(&mut self) -> Result<()> {
        if let Some(edit) = self.undo.pop_back() {
            self.restore(&edit.before)?;
            self.redo.push_back(edit);
        }
        Ok(())
    }

    pub fn redo(&mut self) -> Result<()> {
        if let Some(edit) = self.redo.pop_back() {
            self.restore(&edit.after)?;
            self.undo.push_back(edit);
        }
        Ok(())
    }

    fn add_node(&mut self) -> Result<()> {
        let parent = self.selected;
        let (kind_name, kind) = &KINDS[self.kind];
        let name = (*kind_name).to_owned();
        let kind = *kind;
        let id = self.edit("Add node", |graph| {
            let id = graph.create(parent, kind, name)?;
            if matches!(
                kind,
                NodeKind::Collider2D
                    | NodeKind::Camera2D
                    | NodeKind::Control
                    | NodeKind::Text
                    | NodeKind::AudioSource
            ) {
                NativeDraft::default().apply(graph, id, kind)?;
            }
            Ok(id)
        })?;
        self.select(id)
    }

    fn duplicate(&mut self) -> Result<()> {
        let selected = self.selected;
        let parent = self
            .graph
            .node(selected)?
            .parent()
            .unwrap_or(self.graph.root());
        let id = self.edit("Duplicate node", |graph| graph.duplicate(selected, parent))?;
        self.select(id)
    }

    fn delete(&mut self) -> Result<()> {
        ensure!(
            self.selected != self.graph.root(),
            "scene root cannot be deleted"
        );
        let selected = self.selected;
        let parent = self
            .graph
            .node(selected)?
            .parent()
            .context("node has no parent")?;
        self.edit("Delete node", |graph| graph.destroy(selected))?;
        self.select(parent)
    }

    fn move_sibling(&mut self, direction: isize) -> Result<()> {
        let selected = self.selected;
        let parent = self
            .graph
            .node(selected)?
            .parent()
            .context("scene root has no siblings")?;
        let children = self.graph.node(parent)?.children();
        let current = children
            .iter()
            .position(|child| *child == selected)
            .context("selected node is missing from its parent")?;
        let next = current
            .saturating_add_signed(direction)
            .min(children.len() - 1);
        self.edit("Reorder node", |graph| graph.reorder(selected, next))?;
        Ok(())
    }

    fn move_to_parent(&mut self) -> Result<()> {
        let selected = self.selected;
        let path = self.parent_path.clone();
        self.edit("Reparent node", |graph| {
            let parent = graph
                .find_path(&path)?
                .with_context(|| format!("parent path does not exist: {path}"))?;
            graph.reparent(selected, parent, usize::MAX)
        })?;
        self.select(selected)
    }

    fn move_by_screen_delta(&mut self, id: NodeId, delta: Vec2) -> Result<()> {
        let parent = self.graph.node(id)?.parent();
        let parent_world = parent
            .map(|parent| self.graph.world_matrix(parent))
            .transpose()?
            .unwrap_or(glam::Mat3::IDENTITY);
        ensure!(
            parent_world.determinant().abs() > 0.000001,
            "parent transform cannot be singular"
        );
        let local = parent_world
            .inverse()
            .transform_vector2(glam::Vec2::new(delta.x, delta.y) / self.zoom);
        let mut transform = self.graph.node(id)?.data().transform;
        transform.position[0] += local.x;
        transform.position[1] += local.y;
        if self.snap {
            transform.position[0] = (transform.position[0] / 16.0).round() * 16.0;
            transform.position[1] = (transform.position[1] / 16.0).round() * 16.0;
        }
        self.edit("Move node", |graph| graph.set_transform(id, transform))?;
        self.select(id)
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) -> Result<()> {
        ui.input_mut(|input| {
            if input.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z)
                || input.consume_key(Modifiers::COMMAND, Key::Y)
            {
                if let Err(error) = self.redo() {
                    self.error = Some(format!("{error:#}"));
                }
            } else if input.consume_key(Modifiers::COMMAND, Key::Z) {
                if let Err(error) = self.undo() {
                    self.error = Some(format!("{error:#}"));
                }
            }
        });
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_source("scene_add_kind")
                .selected_text(KINDS[self.kind].0)
                .show_ui(ui, |ui| {
                    for (index, (name, _)) in KINDS.iter().enumerate() {
                        ui.selectable_value(&mut self.kind, index, *name);
                    }
                });
            if ui.button("Add child").clicked() {
                self.capture(Self::add_node);
            }
            if ui.button("Duplicate").clicked() {
                self.capture(Self::duplicate);
            }
            if ui.button("Move up").clicked() {
                self.capture(|this| this.move_sibling(-1));
            }
            if ui.button("Move down").clicked() {
                self.capture(|this| this.move_sibling(1));
            }
            if ui.button("Delete").clicked() {
                self.capture(Self::delete);
            }
            let undo_label = self
                .undo
                .back()
                .map(|edit| edit.label.as_str())
                .unwrap_or("nothing");
            if ui
                .button("Undo")
                .on_hover_text(format!("Undo {undo_label}"))
                .clicked()
            {
                self.capture(Self::undo);
            }
            let redo_label = self
                .redo
                .back()
                .map(|edit| edit.label.as_str())
                .unwrap_or("nothing");
            if ui
                .button("Redo")
                .on_hover_text(format!("Redo {redo_label}"))
                .clicked()
            {
                self.capture(Self::redo);
            }
            ui.separator();
            ui.label(format!("{} nodes", self.graph.len()));
            ui.checkbox(&mut self.snap, "Snap 16");
        });
        if let Some(error) = &self.error {
            ui.colored_label(Color32::LIGHT_RED, error);
        }
        ui.separator();
        ui.columns(3, |columns| {
            columns[0].heading("Scene tree");
            self.tree_ui(&mut columns[0]);
            columns[1].heading("2D viewport");
            self.viewport_ui(&mut columns[1]);
            columns[2].heading("Inspector");
            self.inspector_ui(&mut columns[2]);
        });
        Ok(())
    }

    fn capture(&mut self, action: impl FnOnce(&mut Self) -> Result<()>) {
        if let Err(error) = action(self) {
            self.error = Some(format!("{error:#}"));
        } else {
            self.error = None;
        }
    }

    fn tree_ui(&mut self, ui: &mut egui::Ui) {
        ui.text_edit_singleline(&mut self.filter);
        let mut rows = Vec::new();
        let mut pending = vec![(self.graph.root(), 0_usize)];
        while let Some((id, depth)) = pending.pop() {
            if let Ok(node) = self.graph.node(id) {
                let data = node.data();
                let file_id = data.id;
                let name = data.name.clone();
                let kind = format!("{:?}", data.kind);
                let children = node.children().to_vec();
                rows.push((id, depth, file_id, name, kind, !children.is_empty()));
                if !self.collapsed.contains(&file_id) {
                    pending.extend(children.into_iter().rev().map(|child| (child, depth + 1)));
                }
            }
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (id, depth, file_id, name, kind, has_children) in rows {
                if !self.filter.is_empty()
                    && !name.to_lowercase().contains(&self.filter.to_lowercase())
                {
                    continue;
                }
                ui.horizontal(|ui| {
                    ui.add_space((depth as f32 * 12.0).min(240.0));
                    if has_children {
                        let arrow = if self.collapsed.contains(&file_id) {
                            "▶"
                        } else {
                            "▼"
                        };
                        if ui.small_button(arrow).clicked() && !self.collapsed.remove(&file_id) {
                            self.collapsed.insert(file_id);
                        }
                    } else {
                        ui.add_space(22.0);
                    }
                    if ui
                        .selectable_label(id == self.selected, name)
                        .on_hover_text(kind)
                        .clicked()
                    {
                        self.capture(|this| this.select(id));
                    }
                });
            }
        });
    }

    fn viewport_ui(&mut self, ui: &mut egui::Ui) {
        let size = ui.available_size().max(Vec2::new(200.0, 200.0));
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::from_rgb(20, 23, 29));
        if response.hovered() {
            let scroll = ui.input(|input| input.raw_scroll_delta.y);
            if scroll != 0.0 {
                self.zoom = (self.zoom * (1.0 + scroll * 0.001)).clamp(0.1, 8.0);
            }
        }
        if response.dragged_by(egui::PointerButton::Middle) {
            self.pan += ui.input(|input| input.pointer.delta());
        }
        let origin = rect.center() + self.pan;
        let spacing = 32.0 * self.zoom;
        if spacing >= 8.0 {
            let mut x = (origin.x - rect.left()).rem_euclid(spacing) + rect.left();
            while x < rect.right() {
                painter.line_segment(
                    [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                    Stroke::new(1.0_f32, Color32::from_gray(39)),
                );
                x += spacing;
            }
            let mut y = (origin.y - rect.top()).rem_euclid(spacing) + rect.top();
            while y < rect.bottom() {
                painter.line_segment(
                    [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                    Stroke::new(1.0_f32, Color32::from_gray(39)),
                );
                y += spacing;
            }
        }
        painter.line_segment(
            [
                Pos2::new(origin.x, rect.top()),
                Pos2::new(origin.x, rect.bottom()),
            ],
            Stroke::new(1.0_f32, Color32::from_rgb(110, 55, 55)),
        );
        painter.line_segment(
            [
                Pos2::new(rect.left(), origin.y),
                Pos2::new(rect.right(), origin.y),
            ],
            Stroke::new(1.0_f32, Color32::from_rgb(55, 110, 65)),
        );
        let mut markers = Vec::new();
        let mut outlines = Vec::new();
        let mut pending = vec![self.graph.root()];
        while let Some(id) = pending.pop() {
            if let Ok(node) = self.graph.node(id) {
                let data = node.data().clone();
                let name = data.name.clone();
                let kind = format!("{:?}", data.kind);
                let visible = data.visible;
                let children = node.children().to_vec();
                pending.extend(children.into_iter().rev());
                if visible {
                    if let Ok(matrix) = self.graph.world_matrix(id) {
                        let prop = |key: &str| {
                            data.properties
                                .get(key)
                                .and_then(serde_json::Value::as_f64)
                                .map(|v| v as f32)
                        };
                        let preview = match data.kind {
                            NodeKind::Sprite | NodeKind::AnimatedSprite => Some((
                                prop("width"),
                                prop("height"),
                                Color32::from_rgb(80, 180, 220),
                                false,
                            )),
                            NodeKind::Control => Some((
                                prop("width"),
                                prop("height"),
                                Color32::from_rgb(130, 190, 120),
                                false,
                            )),
                            NodeKind::Collider2D
                                if data
                                    .properties
                                    .get("shape")
                                    .and_then(serde_json::Value::as_str)
                                    != Some("circle") =>
                            {
                                Some((
                                    prop("width"),
                                    prop("height"),
                                    Color32::from_rgb(230, 160, 70),
                                    true,
                                ))
                            }
                            _ => None,
                        };
                        if let Some((Some(w), Some(h), color, centered)) = preview {
                            if w > 0.0 && h > 0.0 {
                                let offset = if centered {
                                    glam::Vec2::new(-w / 2.0, -h / 2.0)
                                } else {
                                    glam::Vec2::ZERO
                                };
                                let corners = [
                                    offset,
                                    offset + glam::Vec2::new(w, 0.0),
                                    offset + glam::Vec2::new(w, h),
                                    offset + glam::Vec2::new(0.0, h),
                                ];
                                let points = corners.map(|point| {
                                    let world = matrix.transform_point2(point);
                                    origin + Vec2::new(world.x, world.y) * self.zoom
                                });
                                outlines.push((points, color));
                            }
                        }
                        if data.kind == NodeKind::Collider2D
                            && data
                                .properties
                                .get("shape")
                                .and_then(serde_json::Value::as_str)
                                == Some("circle")
                        {
                            if let Some(radius) = prop("radius") {
                                let world = matrix.transform_point2(glam::Vec2::ZERO);
                                let center = origin + Vec2::new(world.x, world.y) * self.zoom;
                                painter.circle_stroke(
                                    center,
                                    radius * matrix.x_axis.truncate().length() * self.zoom,
                                    Stroke::new(1.5_f32, Color32::from_rgb(230, 160, 70)),
                                );
                            }
                        }
                        if data.kind == NodeKind::Camera2D
                            && data
                                .properties
                                .get("active")
                                .and_then(serde_json::Value::as_bool)
                                == Some(true)
                        {
                            let zoom = prop("zoom").unwrap_or(1.0);
                            if zoom > 0.0 {
                                let center = self.graph.world_position(id).unwrap_or_default();
                                let axis = matrix.x_axis.truncate();
                                let rotation = glam::Mat2::from_angle(axis.y.atan2(axis.x));
                                let half =
                                    glam::Vec2::new(self.preview_size.x, self.preview_size.y)
                                        / (zoom * 2.0);
                                let corners = [
                                    glam::Vec2::new(-half.x, -half.y),
                                    glam::Vec2::new(half.x, -half.y),
                                    half,
                                    glam::Vec2::new(-half.x, half.y),
                                ];
                                let points = corners.map(|point| {
                                    let world = center + rotation * point;
                                    origin + Vec2::new(world.x, world.y) * self.zoom
                                });
                                outlines.push((points, Color32::LIGHT_BLUE));
                            }
                        }
                    }
                }
                if let Ok(position) = self.graph.world_position(id) {
                    markers.push((
                        id,
                        name,
                        kind,
                        visible,
                        origin + Vec2::new(position.x, position.y) * self.zoom,
                    ));
                }
            }
        }
        for (points, color) in outlines {
            for index in 0..4 {
                painter.line_segment(
                    [points[index], points[(index + 1) % 4]],
                    Stroke::new(1.5_f32, color),
                );
            }
        }
        if response.drag_started_by(egui::PointerButton::Primary) {
            let start = ui.input(|input| input.pointer.press_origin());
            self.dragging = start.and_then(|point| {
                markers
                    .iter()
                    .rev()
                    .find(|(_, _, _, visible, marker)| *visible && marker.distance(point) < 12.0)
                    .map(|row| (row.0, point))
            });
            if let Some((id, _)) = self.dragging {
                self.capture(|this| this.select(id));
            }
        }
        for (id, name, kind, visible, point) in &markers {
            if !rect.contains(*point) || !visible {
                continue;
            }
            let point = if let Some((drag_id, start)) = self.dragging {
                if drag_id == *id && response.dragged_by(egui::PointerButton::Primary) {
                    *point + (response.interact_pointer_pos().unwrap_or(start) - start)
                } else {
                    *point
                }
            } else {
                *point
            };
            let selected = *id == self.selected;
            let color = if selected {
                Color32::YELLOW
            } else if kind == "Camera2D" {
                Color32::LIGHT_BLUE
            } else {
                Color32::LIGHT_GREEN
            };
            painter.circle_stroke(
                point,
                if selected { 8.0 } else { 5.0 },
                Stroke::new(2.0_f32, color),
            );
            painter.text(
                point + Vec2::new(10.0, -8.0),
                egui::Align2::LEFT_TOP,
                name,
                egui::FontId::proportional(12.0),
                color,
            );
        }
        if response.drag_stopped_by(egui::PointerButton::Primary) {
            if let Some((id, start)) = self.dragging.take() {
                let delta = ui
                    .input(|input| input.pointer.latest_pos())
                    .unwrap_or(start)
                    - start;
                self.capture(|this| this.move_by_screen_delta(id, delta));
            }
        }
        if response.clicked() {
            if let Some(pointer) = response.interact_pointer_pos() {
                if let Some((id, _, _, _, _)) = markers
                    .iter()
                    .rev()
                    .find(|(_, _, _, visible, point)| *visible && point.distance(pointer) < 12.0)
                {
                    self.capture(|this| this.select(*id));
                }
            }
        }
        ui.small(format!(
            "Zoom {:.0}% · left drag to move · middle drag to pan",
            self.zoom * 100.0
        ));
    }

    fn native_ui(&mut self, ui: &mut egui::Ui, kind: NodeKind) {
        let has_color = matches!(
            kind,
            NodeKind::Sprite | NodeKind::AnimatedSprite | NodeKind::Control | NodeKind::Text
        );
        match kind {
            NodeKind::Sprite | NodeKind::AnimatedSprite => {
                ui.label("Texture path");
                ui.text_edit_singleline(&mut self.native.path);
                ui.horizontal(|ui| {
                    ui.label("Size");
                    ui.add(egui::DragValue::new(&mut self.native.width));
                    ui.add(egui::DragValue::new(&mut self.native.height));
                });
            }
            NodeKind::Camera2D => {
                ui.checkbox(&mut self.native.active, "Active camera");
                ui.horizontal(|ui| {
                    ui.label("Zoom");
                    ui.add(egui::DragValue::new(&mut self.native.zoom).speed(0.01));
                });
            }
            NodeKind::Collider2D => {
                egui::ComboBox::from_id_source("scene_collider_shape")
                    .selected_text(&self.native.shape)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.native.shape,
                            "rectangle".into(),
                            "Rectangle",
                        );
                        ui.selectable_value(&mut self.native.shape, "circle".into(), "Circle");
                    });
                if self.native.shape == "circle" {
                    ui.horizontal(|ui| {
                        ui.label("Radius");
                        ui.add(egui::DragValue::new(&mut self.native.radius));
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Size");
                        ui.add(egui::DragValue::new(&mut self.native.width));
                        ui.add(egui::DragValue::new(&mut self.native.height));
                    });
                }
            }
            NodeKind::Control => {
                ui.checkbox(&mut self.native.interactive, "Pointer interaction");
                ui.horizontal(|ui| {
                    ui.label("Size");
                    ui.add(egui::DragValue::new(&mut self.native.width));
                    ui.add(egui::DragValue::new(&mut self.native.height));
                });
            }
            NodeKind::Text => {
                ui.label("Text");
                ui.text_edit_singleline(&mut self.native.text);
                ui.horizontal(|ui| {
                    ui.label("Bitmap scale");
                    ui.add(egui::DragValue::new(&mut self.native.scale).speed(0.1));
                });
            }
            NodeKind::AudioSource => {
                ui.label("Sound path");
                ui.text_edit_singleline(&mut self.native.path);
                ui.checkbox(&mut self.native.autoplay, "Autoplay");
                ui.checkbox(&mut self.native.looping, "Loop");
                ui.horizontal(|ui| {
                    ui.label("Volume");
                    ui.add(
                        egui::DragValue::new(&mut self.native.volume)
                            .speed(0.01)
                            .range(0.0..=1.0),
                    );
                });
                egui::ComboBox::from_id_source("scene_audio_bus")
                    .selected_text(&self.native.bus)
                    .show_ui(ui, |ui| {
                        for bus in ["master", "music", "sfx", "ui"] {
                            ui.selectable_value(&mut self.native.bus, bus.into(), bus);
                        }
                    });
            }
            _ => return,
        }
        if has_color {
            ui.horizontal(|ui| {
                ui.label("RGBA");
                for channel in &mut self.native.color {
                    ui.add(egui::DragValue::new(channel).speed(0.01).range(0.0..=1.0));
                }
            });
        }
        if ui.button("Apply native properties").clicked() {
            let selected = self.selected;
            let draft = self.native.clone();
            self.capture(|this| {
                this.edit("Set native properties", |graph| {
                    draft.apply(graph, selected, kind)
                })
                .map(|_| ())
            });
        }
    }

    fn inspector_ui(&mut self, ui: &mut egui::Ui) {
        let Ok(node) = self.graph.node(self.selected) else {
            return;
        };
        let node_kind = node.data().kind;
        let kind = format!("{:?}", node_kind);
        let enabled = node.data().enabled;
        let visible = node.data().visible;
        let properties = node.data().properties.clone();
        ui.label(format!("Type: {kind}"));
        ui.label(format!("ID: {}", node.data().id));
        if self.selected != self.graph.root() {
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.parent_path);
                if ui.button("Move under path").clicked() {
                    self.capture(Self::move_to_parent);
                }
            });
        }
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.name);
            if ui.button("Rename").clicked() {
                let selected = self.selected;
                let name = self.name.clone();
                self.capture(|this| {
                    this.edit("Rename node", |graph| graph.set_name(selected, name))
                        .map(|_| ())
                });
            }
        });
        let mut next_enabled = enabled;
        if ui.checkbox(&mut next_enabled, "Enabled").changed() {
            let selected = self.selected;
            self.capture(|this| {
                this.edit("Set enabled", |graph| {
                    graph.set_enabled(selected, next_enabled)
                })
                .map(|_| ())
            });
        }
        let mut next_visible = visible;
        if ui.checkbox(&mut next_visible, "Visible").changed() {
            let selected = self.selected;
            self.capture(|this| {
                this.edit("Set visible", |graph| {
                    graph.set_visible(selected, next_visible)
                })
                .map(|_| ())
            });
        }
        ui.separator();
        ui.label("Local transform");
        ui.horizontal(|ui| {
            ui.label("Position");
            ui.add(egui::DragValue::new(&mut self.transform.position[0]));
            ui.add(egui::DragValue::new(&mut self.transform.position[1]));
        });
        ui.horizontal(|ui| {
            ui.label("Scale");
            ui.add(egui::DragValue::new(&mut self.transform.scale[0]).speed(0.01));
            ui.add(egui::DragValue::new(&mut self.transform.scale[1]).speed(0.01));
        });
        ui.horizontal(|ui| {
            ui.label("Pivot");
            ui.add(egui::DragValue::new(&mut self.transform.pivot[0]));
            ui.add(egui::DragValue::new(&mut self.transform.pivot[1]));
        });
        ui.horizontal(|ui| {
            ui.label("Rotation (rad)");
            ui.add(egui::DragValue::new(&mut self.transform.rotation).speed(0.01));
        });
        if ui.button("Apply transform").clicked() {
            let selected = self.selected;
            let transform = self.transform;
            self.capture(|this| {
                this.edit("Set transform", |graph| {
                    graph.set_transform(selected, transform)
                })
                .map(|_| ())
            });
        }
        ui.separator();
        self.native_ui(ui, node_kind);
        ui.separator();
        ui.label("Lua script");
        ui.text_edit_singleline(&mut self.script);
        if ui.button("Apply script").clicked() {
            let selected = self.selected;
            let path = (!self.script.is_empty()).then(|| self.script.clone());
            self.capture(|this| {
                this.edit("Set script", |graph| graph.set_script(selected, path))
                    .map(|_| ())
            });
        }
        ui.separator();
        ui.label("Property JSON");
        ui.text_edit_singleline(&mut self.property_name);
        ui.text_edit_singleline(&mut self.property_json);
        if ui.button("Set property").clicked() {
            let selected = self.selected;
            let name = self.property_name.clone();
            let parsed = serde_json::from_str(&self.property_json);
            match parsed {
                Ok(value) => self.capture(|this| {
                    this.edit("Set property", |graph| {
                        graph.set_property(selected, &name, value)
                    })
                    .map(|_| ())
                }),
                Err(error) => self.error = Some(format!("invalid JSON property: {error}")),
            }
        }
        egui::ScrollArea::vertical()
            .max_height(150.0)
            .show(ui, |ui| {
                for (name, value) in properties {
                    ui.label(format!("{name}: {value}"));
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_edits_undo_redo_save_and_reopen_the_runtime_resource() {
        let root = tempfile::tempdir().unwrap();
        let files = ProjectFiles::open(root.path()).unwrap();
        files.create_file("level.scene", b"").unwrap();
        let mut editor = SceneEditor::load(&files, "level.scene".into()).unwrap();
        editor.add_node().unwrap();
        assert_eq!(editor.graph.len(), 2);
        editor.undo().unwrap();
        assert_eq!(editor.graph.len(), 1);
        editor.redo().unwrap();
        assert_eq!(editor.graph.len(), 2);
        editor.save(&files).unwrap();
        assert!(!editor.dirty());
        let reopened = SceneEditor::load(&files, "level.scene".into()).unwrap();
        assert_eq!(reopened.graph.len(), 2);
        assert!(
            SceneGraph::from_json(&files.filesystem().read_text("level.scene").unwrap()).is_ok()
        );
        editor.add_node().unwrap();
        std::fs::write(root.path().join("level.scene"), b"external change").unwrap();
        assert!(editor.save(&files).is_err());
        assert_eq!(files.read("level.scene").unwrap(), b"external change");
    }

    #[test]
    fn native_inspector_properties_and_drag_are_single_undoable_edits() {
        let root = tempfile::tempdir().unwrap();
        let files = ProjectFiles::open(root.path()).unwrap();
        files.create_file("level.scene", b"").unwrap();
        let mut editor = SceneEditor::load(&files, "level.scene".into()).unwrap();
        editor.kind = 8; // Collider2D
        editor.add_node().unwrap();
        let collider = editor.selected;
        assert_eq!(
            editor.graph.node(collider).unwrap().data().properties["width"]
                .as_f64()
                .unwrap(),
            32.0
        );
        editor.native.width = 48.0;
        let draft = editor.native.clone();
        editor
            .edit("Set native properties", |graph| {
                draft.apply(graph, collider, NodeKind::Collider2D)
            })
            .unwrap();
        assert_eq!(
            editor.graph.node(collider).unwrap().data().properties["width"]
                .as_f64()
                .unwrap(),
            48.0
        );
        editor
            .move_by_screen_delta(collider, Vec2::new(20.0, 10.0))
            .unwrap();
        assert_eq!(
            editor
                .graph
                .node(editor.selected)
                .unwrap()
                .data()
                .transform
                .position,
            [20.0, 10.0]
        );
        editor.undo().unwrap();
        assert_eq!(
            editor
                .graph
                .node(editor.selected)
                .unwrap()
                .data()
                .transform
                .position,
            [0.0, 0.0]
        );
        editor.undo().unwrap();
        assert_eq!(
            editor
                .graph
                .node(editor.selected)
                .unwrap()
                .data()
                .properties["width"]
                .as_f64()
                .unwrap(),
            32.0
        );
        editor.redo().unwrap();
        assert_eq!(
            editor
                .graph
                .node(editor.selected)
                .unwrap()
                .data()
                .properties["width"]
                .as_f64()
                .unwrap(),
            48.0
        );
    }

    #[test]
    fn scene_tree_viewport_and_inspector_render_without_a_gpu() {
        let root = tempfile::tempdir().unwrap();
        let files = ProjectFiles::open(root.path()).unwrap();
        files.create_file("level.scene", b"").unwrap();
        let mut editor = SceneEditor::load(&files, "level.scene".into()).unwrap();
        editor.add_node().unwrap();
        let context = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 720.0),
            )),
            ..Default::default()
        };
        let output = context.run(input, |context| {
            egui::CentralPanel::default().show(context, |ui| editor.ui(ui).unwrap());
        });
        assert!(!output.shapes.is_empty());
    }

    #[test]
    fn hierarchy_moves_use_the_same_scene_history_and_reject_cycles() {
        let root = tempfile::tempdir().unwrap();
        let files = ProjectFiles::open(root.path()).unwrap();
        files.create_file("level.scene", b"").unwrap();
        let mut editor = SceneEditor::load(&files, "level.scene".into()).unwrap();
        let scene_root = editor.graph.root();
        let first = editor
            .edit("Add A", |graph| {
                graph.create(scene_root, NodeKind::Node2D, "A")
            })
            .unwrap();
        let second = editor
            .edit("Add B", |graph| {
                graph.create(scene_root, NodeKind::Node2D, "B")
            })
            .unwrap();
        editor.select(first).unwrap();
        editor.move_sibling(1).unwrap();
        assert_eq!(
            editor.graph.node(scene_root).unwrap().children(),
            &[second, first]
        );
        editor.undo().unwrap();
        let first = editor.graph.find_path("level/A").unwrap().unwrap();
        let second = editor.graph.find_path("level/B").unwrap().unwrap();
        assert_eq!(
            editor.graph.node(editor.graph.root()).unwrap().children(),
            &[first, second]
        );
        editor.select(first).unwrap();
        editor.parent_path = "level/B".into();
        editor.move_to_parent().unwrap();
        assert_eq!(editor.graph.node(first).unwrap().parent(), Some(second));
        editor.undo().unwrap();
        let first = editor.graph.find_path("level/A").unwrap().unwrap();
        assert_eq!(
            editor.graph.node(first).unwrap().parent(),
            Some(editor.graph.root())
        );
        editor.select(first).unwrap();
        editor.parent_path = "level/A".into();
        assert!(editor.move_to_parent().is_err());
        assert!(editor.graph.find_path("level/A").unwrap().is_some());
    }
}
