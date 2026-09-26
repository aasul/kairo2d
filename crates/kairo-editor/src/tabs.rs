use crate::{code::CodeTools, document::Document, pixels::Pixels, sprite::SpriteEditor};
use anyhow::{ensure, Result};
use eframe::egui::{self, TextureHandle};
use kairo_audio::AudioEngine;
use kairo_core::SoundHandle;
use kairo_project::ProjectFiles;
use std::io::Cursor;
use std::path::{Path, PathBuf};

pub enum Tab {
    Code(Document),
    Scene(Box<crate::scene_editor::SceneEditor>),
    Animation(Box<crate::animation::AnimationEditor>),
    Sprite(Box<SpriteEditor>),
    Image(ImagePreview),
    Audio(Box<AudioPreview>),
}

impl Tab {
    pub fn open(files: &ProjectFiles, path: PathBuf) -> Result<Self> {
        if path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().ends_with(".anim.json"))
        {
            return Ok(Self::Animation(Box::new(
                crate::animation::AnimationEditor::load(files, path)?,
            )));
        }
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        match extension.as_str() {
            "scene" => Ok(Self::Scene(Box::new(crate::scene_editor::SceneEditor::load(files, path)?))),
            "png" => {
                let bytes = files.read(&path)?;
                let (w, h) = image::io::Reader::new(Cursor::new(&bytes)).with_guessed_format()?.into_dimensions()?;
                if w <= 1024 && h <= 1024 {
                    Ok(Self::Sprite(Box::new(SpriteEditor::new(Pixels::load(files, path)?))))
                } else { Ok(Self::Image(ImagePreview::load(files, path)?)) }
            }
            "jpg" | "jpeg" => Ok(Self::Image(ImagePreview::load(files, path)?)),
            "wav" | "ogg" => Ok(Self::Audio(Box::new(AudioPreview::load(files, path)?))),
            "lua" | "toml" | "txt" | "md" | "json" | "prefab" | "tmj" | "tsj" | "wgsl" | "yaml" | "yml" => Ok(Self::Code(Document::load(files, path)?)),
            _ => anyhow::bail!("unsupported editor file type; supported text, PNG/JPEG and WAV/OGG files can be opened"),
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            Self::Code(doc) => &doc.path,
            Self::Scene(editor) => &editor.path,
            Self::Animation(editor) => &editor.path,
            Self::Sprite(editor) => &editor.pixels.path,
            Self::Image(preview) => &preview.path,
            Self::Audio(preview) => &preview.path,
        }
    }

    pub fn dirty(&self) -> bool {
        match self {
            Self::Animation(editor) => editor.dirty(),
            Self::Code(doc) => doc.dirty(),
            Self::Scene(editor) => editor.dirty(),
            Self::Sprite(editor) => editor.pixels.dirty(),
            _ => false,
        }
    }

    pub fn save(&mut self, files: &ProjectFiles) -> Result<()> {
        match self {
            Self::Animation(editor) => editor.save(files),
            Self::Code(doc) => doc.save(files),
            Self::Scene(editor) => editor.save(files),
            Self::Sprite(editor) => editor.pixels.save(files),
            _ => Ok(()),
        }
    }

    pub fn rename(&mut self, path: PathBuf) {
        match self {
            Self::Code(doc) => doc.path = path,
            Self::Scene(editor) => editor.path = path,
            Self::Animation(editor) => editor.path = path,
            Self::Sprite(editor) => editor.pixels.path = path,
            Self::Image(preview) => preview.path = path,
            Self::Audio(preview) => preview.path = path,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, tools: &mut CodeTools, font_size: f32) -> Result<()> {
        match self {
            Self::Code(doc) => tools.ui(ui, doc, font_size),
            Self::Scene(editor) => editor.ui(ui)?,
            Self::Animation(editor) => editor.ui(ui),
            Self::Sprite(editor) => editor.ui(ui),
            Self::Image(preview) => preview.ui(ui),
            Self::Audio(preview) => preview.ui(ui)?,
        }
        Ok(())
    }
}

pub struct ImagePreview {
    path: PathBuf,
    size: [usize; 2],
    bytes: u64,
    rgba: Vec<u8>,
    texture: Option<TextureHandle>,
    zoom: f32,
}

impl ImagePreview {
    fn load(files: &ProjectFiles, path: PathBuf) -> Result<Self> {
        let bytes = files.read(&path)?;
        let (w, h) = image::io::Reader::new(Cursor::new(&bytes))
            .with_guessed_format()?
            .into_dimensions()?;
        ensure!(
            u64::from(w) * u64::from(h) * 4 <= 64 * 1024 * 1024,
            "image exceeds 64 MiB preview limit"
        );
        let image = image::load_from_memory(&bytes)?.to_rgba8();
        Ok(Self {
            path,
            size: [w as usize, h as usize],
            bytes: bytes.len() as u64,
            rgba: image.into_raw(),
            texture: None,
            zoom: 1.0,
        })
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label(format!(
            "{} x {}    {} bytes    Read-only preview",
            self.size[0], self.size[1], self.bytes
        ));
        ui.add(egui::Slider::new(&mut self.zoom, 0.1..=8.0).text("Zoom"));
        let texture = self.texture.get_or_insert_with(|| {
            ui.ctx().load_texture(
                self.path.display().to_string(),
                egui::ColorImage::from_rgba_unmultiplied(self.size, &self.rgba),
                egui::TextureOptions::NEAREST,
            )
        });
        egui::ScrollArea::both().show(ui, |ui| {
            ui.image((
                texture.id(),
                egui::vec2(self.size[0] as f32, self.size[1] as f32) * self.zoom,
            ));
        });
    }
}

pub struct AudioPreview {
    path: PathBuf,
    engine: AudioEngine,
    sound: SoundHandle,
    duration: f64,
    volume: f32,
    looping: bool,
}

impl AudioPreview {
    fn load(files: &ProjectFiles, path: PathBuf) -> Result<Self> {
        let mut engine = AudioEngine::new(files.filesystem().clone(), true);
        let sound = engine.load(&path.to_string_lossy().replace('\\', "/"))?;
        let duration = engine.duration(sound)?;
        Ok(Self {
            path,
            engine,
            sound,
            duration,
            volume: 0.5,
            looping: false,
        })
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> Result<()> {
        self.engine.prune();
        ui.heading("Audio preview");
        ui.label(self.path.display().to_string());
        ui.label(format!("Duration: {:.2} seconds", self.duration));
        ui.add(egui::Slider::new(&mut self.volume, 0.0..=1.0).text("Volume for next playback"));
        ui.checkbox(&mut self.looping, "Loop");
        if !self.engine.available() {
            ui.label("No audio output device is available.");
        }
        if ui
            .add_enabled(self.engine.available(), egui::Button::new("Play"))
            .clicked()
        {
            self.engine.stop_all();
            self.engine.play(self.sound, self.volume, self.looping)?;
        }
        if ui.button("Stop preview").clicked() {
            self.engine.stop_all();
        }
        Ok(())
    }
}
