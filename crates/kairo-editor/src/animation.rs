use anyhow::{ensure, Context, Result};
use eframe::egui;
use kairo_core::animation::{Animation, AnimationFile, AnimationFrame};
use kairo_project::ProjectFiles;
use std::io::Cursor;
use std::path::PathBuf;

pub struct AnimationEditor {
    pub path: PathBuf,
    file: AnimationFile,
    disk: Option<Vec<u8>>,
    saved: Vec<u8>,
    image: egui::ColorImage,
    texture: Option<egui::TextureHandle>,
    selected: usize,
    cell: [u32; 2],
    duration: f64,
    elapsed: f64,
    playing: bool,
    error: Option<String>,
}
impl AnimationEditor {
    pub fn create(files: &ProjectFiles, path: PathBuf, sheet: &str) -> Result<Self> {
        let texture = PathBuf::from(sheet)
            .file_name()
            .context("select a sprite sheet")?
            .to_str()
            .context("non-UTF-8 sheet name")?
            .to_owned();
        let image = read_image(files, &path, &texture)?;
        let frame = AnimationFrame {
            x: 0,
            y: 0,
            w: image.size[0] as u32,
            h: image.size[1] as u32,
            duration: 0.1,
        };
        Ok(Self {
            path,
            file: AnimationFile {
                texture,
                looping: true,
                frames: vec![frame],
            },
            disk: None,
            saved: Vec::new(),
            image,
            texture: None,
            selected: 0,
            cell: [16, 16],
            duration: 0.1,
            elapsed: 0.0,
            playing: true,
            error: None,
        })
    }
    pub fn load(files: &ProjectFiles, path: PathBuf) -> Result<Self> {
        let bytes = files.read(&path)?;
        ensure!(
            bytes.len() <= 2 * 1024 * 1024,
            "animation metadata exceeds 2 MiB"
        );
        let file: AnimationFile = serde_json::from_slice(&bytes)?;
        ensure!(
            file.frames.len() <= 512,
            "animation editor supports at most 512 frames"
        );
        let image = read_image(files, &path, &file.texture)?;
        Animation::new(
            file.frames.clone(),
            image.size[0] as u32,
            image.size[1] as u32,
        )?;
        let saved = serde_json::to_vec_pretty(&file)?;
        Ok(Self {
            path,
            file,
            disk: Some(bytes),
            saved,
            image,
            texture: None,
            selected: 0,
            cell: [16, 16],
            duration: 0.1,
            elapsed: 0.0,
            playing: true,
            error: None,
        })
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        Animation::new(
            self.file.frames.clone(),
            self.image.size[0] as u32,
            self.image.size[1] as u32,
        )?;
        Ok(serde_json::to_vec_pretty(&self.file)?)
    }
    pub fn dirty(&self) -> bool {
        self.disk.is_none()
            || serde_json::to_vec_pretty(&self.file).is_ok_and(|value| value != self.saved)
    }
    pub fn save(&mut self, files: &ProjectFiles) -> Result<()> {
        // Recheck the on-disk sheet: imports may have changed while this tab was open.
        let image = read_image(files, &self.path, &self.file.texture)?;
        Animation::new(
            self.file.frames.clone(),
            image.size[0] as u32,
            image.size[1] as u32,
        )?;
        let bytes = serde_json::to_vec_pretty(&self.file)?;
        if let Some(previous) = &self.disk {
            ensure!(
                files.read(&self.path)? == *previous,
                "animation changed on disk; reopen before saving"
            );
            files.write(&self.path, &bytes)?;
        } else {
            files.create_file(&self.path, &bytes)?;
        }
        self.saved = bytes.clone();
        self.disk = Some(bytes);
        Ok(())
    }
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label(format!(
            "Sheet: {}  ({} x {})",
            self.file.texture, self.image.size[0], self.image.size[1]
        ));
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.playing, "Play preview");
            ui.checkbox(&mut self.file.looping, "Loop");
            if ui.button("Restart").clicked() {
                self.elapsed = 0.0;
            }
            ui.small("Ctrl/Cmd+S saves animation metadata");
        });
        ui.horizontal(|ui| {
            ui.label("Grid cell");
            ui.add(egui::DragValue::new(&mut self.cell[0]).range(1..=4096));
            ui.add(egui::DragValue::new(&mut self.cell[1]).range(1..=4096));
            ui.add(
                egui::DragValue::new(&mut self.duration)
                    .range(0.001..=10.0)
                    .speed(0.01)
                    .suffix(" s"),
            );
            if ui.button("Slice grid").clicked() {
                let cols = self.image.size[0] / self.cell[0] as usize;
                let rows = self.image.size[1] / self.cell[1] as usize;
                if cols * rows == 0 || cols * rows > 512 {
                    self.error = Some("Grid must contain 1..=512 complete cells".into());
                } else {
                    self.file.frames = (0..rows)
                        .flat_map(|y| (0..cols).map(move |x| (x, y)))
                        .map(|(x, y)| AnimationFrame {
                            x: x as u32 * self.cell[0],
                            y: y as u32 * self.cell[1],
                            w: self.cell[0],
                            h: self.cell[1],
                            duration: self.duration,
                        })
                        .collect();
                    self.selected = 0;
                    self.elapsed = 0.0;
                    self.error = None;
                }
            }
        });
        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut self.selected)
                    .range(0..=self.file.frames.len().saturating_sub(1))
                    .prefix("Frame "),
            );
            if ui
                .add_enabled(
                    self.file.frames.len() < 512,
                    egui::Button::new("Duplicate frame"),
                )
                .clicked()
            {
                if let Some(frame) = self.file.frames.get(self.selected).cloned() {
                    self.file.frames.insert(self.selected + 1, frame);
                    self.selected += 1;
                }
            }
            if ui
                .add_enabled(
                    self.file.frames.len() > 1,
                    egui::Button::new("Remove frame"),
                )
                .clicked()
            {
                self.file.frames.remove(self.selected);
                self.selected = self.selected.min(self.file.frames.len() - 1);
            }
        });
        if let Some(frame) = self.file.frames.get_mut(self.selected) {
            ui.horizontal(|ui| {
                for (label, value) in [
                    ("X", &mut frame.x),
                    ("Y", &mut frame.y),
                    ("W", &mut frame.w),
                    ("H", &mut frame.h),
                ] {
                    ui.label(label);
                    ui.add(egui::DragValue::new(value).range(0..=8192));
                }
                ui.add(
                    egui::DragValue::new(&mut frame.duration)
                        .range(0.001..=10.0)
                        .speed(0.01)
                        .suffix(" s"),
                );
            });
        }
        if self.playing {
            self.elapsed += f64::from(ui.input(|input| input.stable_dt).min(0.1));
            ui.ctx().request_repaint();
        }
        let preview = Animation::new(
            self.file.frames.clone(),
            self.image.size[0] as u32,
            self.image.size[1] as u32,
        )
        .and_then(|mut animation| {
            animation.set_looping(self.file.looping);
            animation.update(self.elapsed)?;
            Ok(animation)
        });
        match preview {
            Ok(animation) => {
                let frame = if self.playing {
                    animation.frame()
                } else {
                    &self.file.frames[self.selected]
                };
                let texture = self.texture.get_or_insert_with(|| {
                    ui.ctx().load_texture(
                        self.path.display().to_string(),
                        self.image.clone(),
                        egui::TextureOptions::NEAREST,
                    )
                });
                let uv = egui::Rect::from_min_max(
                    egui::pos2(
                        frame.x as f32 / self.image.size[0] as f32,
                        frame.y as f32 / self.image.size[1] as f32,
                    ),
                    egui::pos2(
                        (frame.x + frame.w) as f32 / self.image.size[0] as f32,
                        (frame.y + frame.h) as f32 / self.image.size[1] as f32,
                    ),
                );
                let size = egui::vec2(frame.w as f32, frame.h as f32);
                let scale = (240.0 / size.x.max(size.y)).clamp(0.1, 16.0);
                ui.add(egui::Image::new((texture.id(), size * scale)).uv(uv));
            }
            Err(error) => {
                ui.colored_label(egui::Color32::LIGHT_RED, error.to_string());
            }
        }
        if let Some(error) = &self.error {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
    }
}
fn read_image(
    files: &ProjectFiles,
    path: &std::path::Path,
    reference: &str,
) -> Result<egui::ColorImage> {
    let path = path
        .to_str()
        .context("non-UTF-8 animation path")?
        .replace('\\', "/");
    let reference = kairo_assets::tilemap::relative_asset(&path, reference)?;
    let bytes = files.read(reference)?;
    let (width, height) = image::io::Reader::new(Cursor::new(&bytes))
        .with_guessed_format()?
        .into_dimensions()?;
    ensure!(
        width > 0 && height > 0 && u64::from(width) * u64::from(height) * 4 <= 64 * 1024 * 1024,
        "sprite sheet exceeds preview budget"
    );
    let image = image::load_from_memory(&bytes)?.to_rgba8();
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [width as usize, height as usize],
        image.as_raw(),
    ))
}
