use anyhow::{ensure, Context, Result};
use kairo_project::ProjectFiles;
use std::collections::VecDeque;
use std::io::Cursor;
use std::path::PathBuf;

const MAX_SIDE: u32 = 1024;
const HISTORY_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, PartialEq)]
struct PixelSnapshot {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

pub struct PixelBlock {
    pub width: u32,
    pub height: u32,
    rgba: Vec<u8>,
}

pub struct Pixels {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub revision: u64,
    saved: PixelSnapshot,
    disk: Option<Vec<u8>>,
    stroke: Option<PixelSnapshot>,
    undo: VecDeque<PixelSnapshot>,
    redo: Vec<PixelSnapshot>,
}

impl Pixels {
    pub fn new(path: PathBuf, width: u32, height: u32) -> Result<Self> {
        ensure!(
            (1..=MAX_SIDE).contains(&width) && (1..=MAX_SIDE).contains(&height),
            "sprite dimensions must be in 1..=1024"
        );
        Ok(Self {
            path,
            width,
            height,
            rgba: vec![0; width as usize * height as usize * 4],
            revision: 0,
            saved: PixelSnapshot {
                width,
                height,
                rgba: Vec::new(),
            },
            disk: None,
            stroke: None,
            undo: VecDeque::new(),
            redo: Vec::new(),
        })
    }

    pub fn load(files: &ProjectFiles, path: PathBuf) -> Result<Self> {
        let disk = files.read(&path)?;
        let (width, height) = image::io::Reader::new(Cursor::new(&disk))
            .with_guessed_format()?
            .into_dimensions()?;
        let mut pixels = Self::new(path, width, height)?;
        pixels.rgba = image::load_from_memory(&disk)?.to_rgba8().into_raw();
        pixels.saved = pixels.snapshot();
        pixels.disk = Some(disk);
        Ok(pixels)
    }

    pub fn dirty(&self) -> bool {
        self.disk.is_none()
            || self.width != self.saved.width
            || self.height != self.saved.height
            || self.rgba != self.saved.rgba
    }

    fn snapshot(&self) -> PixelSnapshot {
        PixelSnapshot {
            width: self.width,
            height: self.height,
            rgba: self.rgba.clone(),
        }
    }

    fn restore(&mut self, snapshot: PixelSnapshot) -> PixelSnapshot {
        let previous = PixelSnapshot {
            width: self.width,
            height: self.height,
            rgba: std::mem::replace(&mut self.rgba, snapshot.rgba),
        };
        self.width = snapshot.width;
        self.height = snapshot.height;
        self.revision = self.revision.wrapping_add(1);
        previous
    }

    pub fn begin(&mut self) {
        if self.stroke.is_none() {
            self.stroke = Some(self.snapshot());
        }
    }

    pub fn commit(&mut self) {
        if let Some(before) = self.stroke.take() {
            if before.width != self.width
                || before.height != self.height
                || before.rgba != self.rgba
            {
                self.undo.push_back(before);
                while self.undo.len() > 32
                    || self.undo.iter().map(|s| s.rgba.len()).sum::<usize>() > HISTORY_BYTES
                {
                    self.undo.pop_front();
                }
                self.redo.clear();
            }
        }
    }

    pub fn undo(&mut self) {
        self.commit();
        if let Some(before) = self.undo.pop_back() {
            let previous = self.restore(before);
            self.redo.push(previous);
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            let previous = self.restore(next);
            self.undo.push_back(previous);
        }
    }

    pub fn color(&self, x: i32, y: i32) -> Option<[u8; 4]> {
        self.offset(x, y).map(|i| {
            [
                self.rgba[i],
                self.rgba[i + 1],
                self.rgba[i + 2],
                self.rgba[i + 3],
            ]
        })
    }

    fn offset(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        Some((y as usize * self.width as usize + x as usize) * 4)
    }

    pub fn set(&mut self, x: i32, y: i32, color: [u8; 4]) {
        if let Some(i) = self.offset(x, y) {
            if self.rgba[i..i + 4] != color {
                self.rgba[i..i + 4].copy_from_slice(&color);
                self.revision = self.revision.wrapping_add(1);
            }
        }
    }

    pub fn line(&mut self, from: (i32, i32), to: (i32, i32), color: [u8; 4]) {
        let (mut x, mut y) = from;
        let (x1, y1) = to;
        let dx = (x1 - x).abs();
        let dy = -(y1 - y).abs();
        let sx = if x < x1 { 1 } else { -1 };
        let sy = if y < y1 { 1 } else { -1 };
        let mut error = dx + dy;
        loop {
            self.set(x, y, color);
            if x == x1 && y == y1 {
                break;
            }
            let twice = 2 * error;
            if twice >= dy {
                error += dy;
                x += sx;
            }
            if twice <= dx {
                error += dx;
                y += sy;
            }
        }
    }

    pub fn fill(&mut self, x: i32, y: i32, replacement: [u8; 4]) {
        let Some(original) = self.color(x, y) else {
            return;
        };
        if original == replacement {
            return;
        }
        let mut pending = vec![(x, y)];
        self.set(x, y, replacement);
        while let Some((x, y)) = pending.pop() {
            for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
                if self.color(nx, ny) == Some(original) {
                    // Mark on insertion so each pixel enters the stack at most once.
                    self.set(nx, ny, replacement);
                    pending.push((nx, ny));
                }
            }
        }
    }

    pub fn flip(&mut self, horizontal: bool) {
        self.begin();
        let before = self.rgba.clone();
        for y in 0..self.height {
            for x in 0..self.width {
                let sx = if horizontal { self.width - x - 1 } else { x };
                let sy = if horizontal { y } else { self.height - y - 1 };
                let source = ((sy * self.width + sx) * 4) as usize;
                let target = ((y * self.width + x) * 4) as usize;
                self.rgba[target..target + 4].copy_from_slice(&before[source..source + 4]);
            }
        }
        self.revision = self.revision.wrapping_add(1);
        self.commit();
    }

    pub fn rectangle(&mut self, from: (i32, i32), to: (i32, i32), color: [u8; 4]) {
        let left = from.0.min(to.0).clamp(0, self.width as i32 - 1);
        let right = from.0.max(to.0).clamp(0, self.width as i32 - 1);
        let top = from.1.min(to.1).clamp(0, self.height as i32 - 1);
        let bottom = from.1.max(to.1).clamp(0, self.height as i32 - 1);
        self.line((left, top), (right, top), color);
        self.line((left, bottom), (right, bottom), color);
        self.line((left, top), (left, bottom), color);
        self.line((right, top), (right, bottom), color);
    }

    pub fn copy_region(&self, from: (i32, i32), to: (i32, i32)) -> PixelBlock {
        let left = from.0.min(to.0).clamp(0, self.width as i32 - 1) as u32;
        let right = from.0.max(to.0).clamp(0, self.width as i32 - 1) as u32;
        let top = from.1.min(to.1).clamp(0, self.height as i32 - 1) as u32;
        let bottom = from.1.max(to.1).clamp(0, self.height as i32 - 1) as u32;
        let width = right - left + 1;
        let height = bottom - top + 1;
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for y in top..=bottom {
            let offset = ((y * self.width + left) * 4) as usize;
            rgba.extend_from_slice(&self.rgba[offset..offset + width as usize * 4]);
        }
        PixelBlock {
            width,
            height,
            rgba,
        }
    }

    pub fn paste(&mut self, block: &PixelBlock, at: (i32, i32)) {
        self.begin();
        for y in 0..block.height {
            for x in 0..block.width {
                let offset = ((y * block.width + x) * 4) as usize;
                let color = [
                    block.rgba[offset],
                    block.rgba[offset + 1],
                    block.rgba[offset + 2],
                    block.rgba[offset + 3],
                ];
                self.set(at.0 + x as i32, at.1 + y as i32, color);
            }
        }
        self.commit();
    }

    /// Resizing crops or pads at the bottom/right; it never resamples pixel art.
    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        ensure!(
            (1..=MAX_SIDE).contains(&width) && (1..=MAX_SIDE).contains(&height),
            "sprite dimensions must be 1..=1024"
        );
        if width == self.width && height == self.height {
            return Ok(());
        }
        self.begin();
        let mut rgba = vec![0; (width * height * 4) as usize];
        for y in 0..height.min(self.height) {
            let from = (y * self.width * 4) as usize;
            let to = (y * width * 4) as usize;
            let length = (width.min(self.width) * 4) as usize;
            rgba[to..to + length].copy_from_slice(&self.rgba[from..from + length]);
        }
        self.rgba = rgba;
        self.width = width;
        self.height = height;
        self.revision = self.revision.wrapping_add(1);
        self.commit();
        Ok(())
    }

    pub fn encode_png(&self) -> Result<Vec<u8>> {
        let image = image::RgbaImage::from_raw(self.width, self.height, self.rgba.clone())
            .context("invalid pixel buffer")?;
        let mut encoded = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut encoded, image::ImageOutputFormat::Png)?;
        Ok(encoded.into_inner())
    }

    pub fn save(&mut self, files: &ProjectFiles) -> Result<()> {
        self.commit();
        ensure!(
            self.path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("png")),
            "sprites must be saved as PNG"
        );
        let png = self.encode_png()?;
        if let Some(original) = &self.disk {
            ensure!(
                files.read(&self.path)? == *original,
                "sprite changed on disk; reopen it before saving"
            );
            files.write(&self.path, &png)?;
        } else {
            files.create_file(&self.path, &png)?;
        }
        self.disk = Some(png);
        self.saved = self.snapshot();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_edits_undo_and_save_as_a_real_png() {
        let root = tempfile::tempdir().unwrap();
        let files = ProjectFiles::open(root.path()).unwrap();
        files.create_directory("assets").unwrap();
        let mut sprite = Pixels::new("assets/test.png".into(), 8, 8).unwrap();
        sprite.begin();
        sprite.line((0, 0), (7, 7), [255, 0, 0, 255]);
        sprite.commit();
        assert_eq!(sprite.color(3, 3), Some([255, 0, 0, 255]));
        sprite.undo();
        assert_eq!(sprite.color(3, 3), Some([0; 4]));
        sprite.redo();
        sprite.save(&files).unwrap();
        let image = image::load_from_memory(&files.read("assets/test.png").unwrap())
            .unwrap()
            .to_rgba8();
        assert_eq!(image.dimensions(), (8, 8));
        assert_eq!(image.get_pixel(3, 3).0, [255, 0, 0, 255]);
        assert_eq!(image.get_pixel(3, 4).0, [0; 4]);
        assert!(!sprite.dirty());
    }

    #[test]
    fn fill_respects_barriers_and_flip_is_undoable() {
        let mut sprite = Pixels::new("test.png".into(), 5, 5).unwrap();
        sprite.begin();
        sprite.line((2, 0), (2, 4), [255; 4]);
        sprite.fill(0, 0, [10, 20, 30, 255]);
        sprite.commit();
        assert_eq!(sprite.color(4, 0), Some([0; 4]));
        sprite.flip(true);
        assert_eq!(sprite.color(4, 0), Some([10, 20, 30, 255]));
        sprite.undo();
        assert_eq!(sprite.color(4, 0), Some([0; 4]));
    }

    #[test]
    fn invalid_dimensions_and_outside_pixels_are_safe() {
        assert!(Pixels::new("bad.png".into(), 0, 10).is_err());
        assert!(Pixels::new("bad.png".into(), 2048, 2048).is_err());
        let mut sprite = Pixels::new("test.png".into(), 1, 1).unwrap();
        sprite.set(-1, 0, [255; 4]);
        assert_eq!(sprite.rgba, [0; 4]);
    }
}

#[cfg(test)]
mod resize_tests {
    use super::*;
    #[test]
    fn resizing_and_pasting_preserve_history_dimensions() {
        let mut p = Pixels::new("a.png".into(), 3, 2).unwrap();
        p.set(1, 1, [255, 10, 20, 255]);
        let block = p.copy_region((1, 1), (1, 1));
        p.resize(6, 4).unwrap();
        p.paste(&block, (5, 3));
        assert_eq!(p.color(5, 3), Some([255, 10, 20, 255]));
        p.undo();
        p.undo();
        assert_eq!((p.width, p.height), (3, 2));
        assert_eq!(p.color(1, 1), Some([255, 10, 20, 255]));
        p.redo();
        assert_eq!((p.width, p.height), (6, 4));
    }
}
