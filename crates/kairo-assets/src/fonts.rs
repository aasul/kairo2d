//! Rasterizes glyphs on demand into reusable RGBA atlas pages with fixed size limits.
use crate::{AssetManager, TextureFilter};
use anyhow::{ensure, Context, Result};
use fontdue::{Font, FontSettings, Metrics};
use kairo_core::{FontHandle, ProjectFs, TextureHandle};
use std::collections::HashMap;
use std::path::PathBuf;

const SIDE: u32 = 1024;
const MAX_PAGES: usize = 8;

#[derive(Clone, Copy, Debug)]
pub struct GlyphQuad {
    pub texture: TextureHandle,
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
}

pub struct TextLayout {
    pub glyphs: Vec<GlyphQuad>,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy)]
struct CachedGlyph {
    metrics: Metrics,
    atlas: Option<(TextureHandle, u32, u32)>,
}

struct Page {
    handle: TextureHandle,
    x: u32,
    y: u32,
    row: u32,
}

struct FontAsset {
    font: Font,
    size: f32,
    ascent: f32,
    line_height: f32,
    glyphs: HashMap<char, CachedGlyph>,
    pages: Vec<Page>,
}

pub struct FontManager {
    fs: ProjectFs,
    fonts: HashMap<FontHandle, FontAsset>,
    paths: HashMap<(PathBuf, u32), FontHandle>,
}

impl FontManager {
    pub fn new(fs: ProjectFs) -> Self {
        Self {
            fs,
            fonts: HashMap::new(),
            paths: HashMap::new(),
        }
    }

    pub fn load(&mut self, relative: &str, size: f32) -> Result<FontHandle> {
        ensure!(
            size.is_finite() && (4.0..=256.0).contains(&size),
            "font size must be in 4..=256 pixels"
        );
        let key = (self.fs.resolve(relative)?, size.to_bits());
        if let Some(handle) = self.paths.get(&key) {
            return Ok(*handle);
        }
        ensure!(
            self.fonts.len() < 32,
            "font cache limit is 32 font/size pairs"
        );
        let bytes = self.fs.read(relative)?;
        ensure!(bytes.len() <= 16 * 1024 * 1024, "font file exceeds 16 MiB");
        let font = Font::from_bytes(bytes, FontSettings::default())
            .map_err(|error| anyhow::anyhow!("cannot parse font {relative}: {error}"))?;
        let metrics = font
            .horizontal_line_metrics(size)
            .context("font has no horizontal line metrics")?;
        let handle = FontHandle::allocate()?;
        self.fonts.insert(
            handle,
            FontAsset {
                font,
                size,
                ascent: metrics.ascent,
                line_height: metrics.new_line_size.max(size),
                glyphs: HashMap::new(),
                pages: Vec::new(),
            },
        );
        self.paths.insert(key, handle);
        Ok(handle)
    }

    pub fn release(&mut self, handle: FontHandle, textures: &mut AssetManager) -> Result<()> {
        let asset = self
            .fonts
            .remove(&handle)
            .context("unknown or released font")?;
        self.paths.retain(|_, cached| *cached != handle);
        for page in asset.pages {
            textures.release(page.handle)?;
        }
        Ok(())
    }

    pub fn count(&self) -> usize {
        self.fonts.len()
    }

    pub fn layout(
        &mut self,
        textures: &mut AssetManager,
        handle: FontHandle,
        text: &str,
    ) -> Result<TextLayout> {
        ensure!(
            text.len() <= 16 * 1024 && text.chars().count() <= 4096,
            "text is limited to 4096 characters and 16 KiB per call"
        );
        let font = self
            .fonts
            .get_mut(&handle)
            .context("unknown or released font")?;
        let mut glyphs = Vec::new();
        let mut cursor = [0.0_f32, font.ascent];
        let mut width = 0.0_f32;
        let mut lines = 1;
        let mut previous = None;
        for character in text.chars() {
            if character == '\r' {
                continue;
            }
            if character == '\n' {
                width = width.max(cursor[0]);
                cursor[0] = 0.0;
                cursor[1] += font.line_height;
                previous = None;
                lines += 1;
                continue;
            }
            if character == '\t' {
                cursor[0] += font.font.metrics(' ', font.size).advance_width * 4.0;
                previous = None;
                continue;
            }
            if let Some(left) = previous {
                cursor[0] += font
                    .font
                    .horizontal_kern(left, character, font.size)
                    .unwrap_or(0.0);
            }
            let cached = font.glyph(textures, character)?;
            let m = cached.metrics;
            if let Some((texture, x, y)) = cached.atlas {
                glyphs.push(GlyphQuad {
                    texture,
                    position: [
                        cursor[0] + m.xmin as f32,
                        cursor[1] - m.ymin as f32 - m.height as f32,
                    ],
                    size: [m.width as f32, m.height as f32],
                    uv_min: [x as f32 / SIDE as f32, y as f32 / SIDE as f32],
                    uv_max: [
                        (x as f32 + m.width as f32) / SIDE as f32,
                        (y as f32 + m.height as f32) / SIDE as f32,
                    ],
                });
            }
            cursor[0] += m.advance_width;
            previous = Some(character);
        }
        Ok(TextLayout {
            glyphs,
            width: width.max(cursor[0]),
            height: lines as f32 * font.line_height,
        })
    }
}

impl FontAsset {
    fn glyph(&mut self, textures: &mut AssetManager, character: char) -> Result<CachedGlyph> {
        if let Some(cached) = self.glyphs.get(&character) {
            return Ok(*cached);
        }
        ensure!(self.glyphs.len() < 8192, "font glyph limit exceeded (8192)");
        let (metrics, coverage) = self.font.rasterize(character, self.size);
        let mut glyph = CachedGlyph {
            metrics,
            atlas: None,
        };
        if metrics.width != 0 && metrics.height != 0 {
            let (w, h) = (
                u32::try_from(metrics.width)?,
                u32::try_from(metrics.height)?,
            );
            ensure!(
                w + 2 < SIDE && h + 2 < SIDE,
                "glyph is larger than an atlas page"
            );
            let mut slot = None;
            if let Some(page) = self.pages.last_mut() {
                if page.x + w + 1 >= SIDE {
                    page.x = 1;
                    page.y += page.row + 1;
                    page.row = 0;
                }
                if page.y + h + 1 < SIDE {
                    slot = Some((page.handle, page.x, page.y));
                }
            }
            if slot.is_none() {
                ensure!(
                    self.pages.len() < MAX_PAGES,
                    "font atlas page budget exhausted"
                );
                // Transparent padding keeps white RGB so linear sampling does not darken glyph edges.
                let mut pixels = vec![255; SIDE as usize * SIDE as usize * 4];
                for alpha in pixels.iter_mut().skip(3).step_by(4) {
                    *alpha = 0;
                }
                let handle = textures.insert_pixels(SIDE, SIDE, pixels, TextureFilter::Linear)?;
                self.pages.push(Page {
                    handle,
                    x: 1,
                    y: 1,
                    row: 0,
                });
                slot = Some((handle, 1, 1));
            }
            let (handle, x, y) = slot.context("glyph allocation failed")?;
            let mut rgba = Vec::with_capacity(coverage.len() * 4);
            for alpha in coverage {
                rgba.extend_from_slice(&[255, 255, 255, alpha]);
            }
            textures.write_region(handle, x, y, w, h, &rgba)?;
            let page = self.pages.last_mut().context("font atlas is missing")?;
            page.x = x + w + 1;
            page.row = page.row.max(h);
            glyph.atlas = Some((handle, x, y));
        }
        self.glyphs.insert(character, glyph);
        Ok(glyph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_fonts_and_invalid_sizes_do_not_enter_cache() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("bad.ttf"), b"not a font").unwrap();
        let mut fonts = FontManager::new(ProjectFs::new(root.path()).unwrap());
        assert!(fonts.load("bad.ttf", 24.0).is_err());
        assert!(fonts.load("bad.ttf", f32::NAN).is_err());
        assert!(fonts.load("../bad.ttf", 24.0).is_err());
        assert_eq!(fonts.count(), 0);
    }
}
