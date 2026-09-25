use crate::workspace::Workspace;
use anyhow::{ensure, Context, Result};
use eframe::egui;
use kairo_core::particles::{Emitter, ParticlePreset};
use std::path::{Path, PathBuf};

pub struct ParticleTool {
    path: String,
    preset: ParticlePreset,
    emitter: Emitter,
    disk: Option<Vec<u8>>,
    saved: String,
    texture: Option<egui::TextureHandle>,
    loaded_texture: String,
    running: bool,
    burst: usize,
    message: String,
}
impl ParticleTool {
    pub fn new() -> Result<Self> {
        let preset = ParticlePreset::default();
        let saved = toml::to_string(&preset)?;
        Ok(Self {
            path: "assets/sparks.particle.toml".into(),
            emitter: Emitter::new(preset.emitter.clone())?,
            preset,
            disk: None,
            saved,
            texture: None,
            loaded_texture: String::new(),
            running: true,
            burst: 32,
            message: String::new(),
        })
    }
    pub fn dirty(&self) -> bool {
        toml::to_string(&self.preset).is_ok_and(|v| v != self.saved)
    }
    pub fn save(&mut self, workspace: &mut Workspace) -> Result<()> {
        ensure!(
            self.path.ends_with(".particle.toml"),
            "particle presets use .particle.toml"
        );
        self.preset.emitter.validate()?;
        if let Some(texture) = &self.preset.texture {
            workspace.files.read(self.texture_path(texture)?)?;
        }
        let bytes = toml::to_string_pretty(&self.preset)?.into_bytes();
        workspace.save_tool_file(Path::new(&self.path), self.disk.as_deref(), &bytes)?;
        self.disk = Some(bytes);
        self.saved = toml::to_string(&self.preset)?;
        self.message = "Preset saved. Load it in Lua with particles.load(path).".into();
        Ok(())
    }
    fn texture_path(&self, texture: &str) -> Result<PathBuf> {
        Ok(PathBuf::from(kairo_assets::tilemap::relative_asset(
            &self.path, texture,
        )?))
    }
    fn load(&mut self, workspace: &Workspace) -> Result<()> {
        let bytes = workspace.files.read(&self.path)?;
        ensure!(bytes.len() < 64 * 1024, "particle preset exceeds 64 KiB");
        let preset: ParticlePreset = toml::from_str(std::str::from_utf8(&bytes)?)?;
        let emitter = Emitter::new(preset.emitter.clone())?;
        self.saved = toml::to_string(&preset)?;
        self.preset = preset;
        self.emitter = emitter;
        self.disk = Some(bytes);
        self.loaded_texture.clear();
        self.texture = None;
        self.message = "Loaded preset.".into();
        Ok(())
    }
    fn texture(&mut self, ctx: &egui::Context, workspace: &Workspace) -> Result<()> {
        let Some(name) = &self.preset.texture else {
            self.texture = None;
            self.loaded_texture.clear();
            return Ok(());
        };
        if *name == self.loaded_texture {
            return Ok(());
        }
        let name = name.clone();
        self.loaded_texture = name.clone();
        self.texture = None;
        let bytes = workspace.files.read(self.texture_path(&name)?)?;
        ensure!(
            bytes.len() <= 16 * 1024 * 1024,
            "preview texture exceeds 16 MiB"
        );
        let reader = image::io::Reader::new(std::io::Cursor::new(&bytes)).with_guessed_format()?;
        let dimensions = reader.into_dimensions()?;
        ensure!(
            dimensions.0 <= 4096 && dimensions.1 <= 4096,
            "preview texture must fit 4096 x 4096"
        );
        let image = image::load_from_memory(&bytes)
            .context("cannot decode particle texture")?
            .to_rgba8();
        let size = [image.width() as usize, image.height() as usize];
        self.texture = Some(ctx.load_texture(
            "particle-preview",
            egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
            egui::TextureOptions::NEAREST,
        ));
        Ok(())
    }
    pub fn ui(&mut self, ui: &mut egui::Ui, workspace: &mut Workspace) -> Result<()> {
        ui.label("Native particle preview - uses the same simulator as the game runtime");
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    self.disk.is_none(),
                    egui::TextEdit::singleline(&mut self.path)
                        .hint_text("assets/effect.particle.toml"),
                )
                .changed()
            {
                self.loaded_texture.clear();
            }
            if ui.button("Save").clicked() {
                if let Err(e) = self.save(workspace) {
                    self.message = format!("{e:#}");
                }
            }
            if ui.button("Load / discard edits").clicked() {
                if let Err(e) = self.load(workspace) {
                    self.message = format!("{e:#}");
                }
            }
            if ui.button("New preset").clicked() {
                self.disk = None;
                self.loaded_texture.clear();
                self.message = "Choose a new, unused path before saving.".into();
            }
        });
        ui.label(&self.message);
        let old = toml::to_string(&self.preset)?;
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.running, "Play");
            ui.add(
                egui::DragValue::new(&mut self.burst)
                    .range(1..=8192)
                    .prefix("Burst "),
            );
            if ui.button("Emit burst").clicked() {
                self.emitter.emit(self.burst);
            }
            if ui.button("Clear").clicked() {
                self.emitter.clear();
            }
            ui.label(format!("{} particles", self.emitter.particles().len()));
        });
        let config = &mut self.preset.emitter;
        egui::Grid::new("particle-controls")
            .num_columns(2)
            .show(ui, |ui| {
                for (name, value, range) in [
                    ("Rate / second", &mut config.rate, 0.0..=1000.0),
                    ("Gravity", &mut config.gravity, -2000.0..=2000.0),
                    ("Drag", &mut config.drag, 0.0..=20.0),
                    (
                        "Direction (rad)",
                        &mut config.direction,
                        -std::f32::consts::TAU..=std::f32::consts::TAU,
                    ),
                    (
                        "Spread (rad)",
                        &mut config.spread,
                        0.0..=std::f32::consts::TAU,
                    ),
                ] {
                    ui.label(name);
                    ui.add(egui::Slider::new(value, range));
                    ui.end_row();
                }
                for (name, range, values) in [
                    ("Lifetime (s)", 0.01..=10.0, &mut config.lifetime),
                    ("Speed", 0.0..=2000.0, &mut config.speed),
                    ("Size start/end", 0.0..=128.0, &mut config.size),
                    (
                        "Rotation (rad)",
                        -std::f32::consts::PI..=std::f32::consts::PI,
                        &mut config.rotation,
                    ),
                ] {
                    ui.label(name);
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut values[0])
                                .range(range.clone())
                                .speed(0.05),
                        );
                        ui.add(
                            egui::DragValue::new(&mut values[1])
                                .range(range)
                                .speed(0.05),
                        );
                    });
                    ui.end_row();
                }
                ui.label("Start color");
                ui.color_edit_button_rgba_unmultiplied(&mut config.color_start);
                ui.end_row();
                ui.label("End color");
                ui.color_edit_button_rgba_unmultiplied(&mut config.color_end);
                ui.end_row();
                ui.label("Capacity");
                ui.add(egui::DragValue::new(&mut config.max_particles).range(1..=8192));
                ui.end_row();
            });
        let mut texture = self.preset.texture.clone().unwrap_or_default();
        ui.horizontal(|ui| {
            ui.label("Texture relative to preset (optional)");
            if ui.text_edit_singleline(&mut texture).changed() {
                self.preset.texture = if texture.is_empty() {
                    None
                } else {
                    Some(texture.clone())
                };
            }
            if ui.button("Reload image").clicked() {
                self.loaded_texture.clear();
            }
        });
        if toml::to_string(&self.preset)? != old {
            match self.emitter.reconfigure(self.preset.emitter.clone()) {
                Ok(()) => self.message = "Modified preset".into(),
                Err(e) => self.message = e.to_string(),
            }
        }
        let size = egui::vec2(ui.available_width().max(100.0), 260.0);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
        ui.painter()
            .rect_filled(rect, 2.0, egui::Color32::from_gray(14));
        if let Some(pos) = response.interact_pointer_pos() {
            if response.dragged() || response.clicked() {
                self.emitter
                    .set_position(pos.x - rect.center().x, pos.y - rect.center().y)?;
            }
        }
        if self.running {
            self.emitter
                .update(ui.input(|i| i.stable_dt).clamp(0.0, 0.05))?;
        }
        if let Err(e) = self.texture(ui.ctx(), workspace) {
            self.message = e.to_string();
        }
        let painter = ui.painter().with_clip_rect(rect);
        for p in self.emitter.particles() {
            let (size, color) = self.emitter.appearance(p);
            let tint = egui::Color32::from_rgba_unmultiplied(
                (color[0] * 255.0) as u8,
                (color[1] * 255.0) as u8,
                (color[2] * 255.0) as u8,
                (color[3] * 255.0) as u8,
            );
            let mut mesh = egui::Mesh::with_texture(
                self.texture
                    .as_ref()
                    .map_or(egui::TextureId::default(), egui::TextureHandle::id),
            );
            let center = rect.center() + egui::vec2(p.position.x, p.position.y);
            let rot = egui::emath::Rot2::from_angle(p.rotation);
            for (corner, uv) in [
                (egui::vec2(-0.5, -0.5), egui::pos2(0.0, 0.0)),
                (egui::vec2(0.5, -0.5), egui::pos2(1.0, 0.0)),
                (egui::vec2(0.5, 0.5), egui::pos2(1.0, 1.0)),
                (egui::vec2(-0.5, 0.5), egui::pos2(0.0, 1.0)),
            ] {
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: center + rot * (corner * size),
                    uv: if self.texture.is_some() {
                        uv
                    } else {
                        egui::epaint::WHITE_UV
                    },
                    color: tint,
                });
            }
            mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
            painter.add(egui::Shape::mesh(mesh));
        }
        ui.small("Click or drag in the preview to move the emitter. Sizes are square; alpha blending only.");
        if self.running {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(16));
        }
        Ok(())
    }
}
