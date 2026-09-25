//! Texture decoding and path-based caching. GPU objects live in kairo-render.
pub mod fonts;
pub mod tilemap;

use anyhow::{ensure, Context, Result};
use kairo_core::{ProjectFs, TextureHandle};
use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};

const MAX_TEXTURE_BYTES: usize = 64 * 1024 * 1024;
const MAX_CACHE_BYTES: usize = 256 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextureFilter {
    #[default]
    Nearest,
    Linear,
}

pub struct TextureAsset {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub filter: TextureFilter,
    pub revision: u64,
}

pub struct AssetManager {
    fs: ProjectFs,
    textures: HashMap<TextureHandle, TextureAsset>,
    paths: HashMap<PathBuf, TextureHandle>,
    texture_bytes: usize,
}

impl AssetManager {
    pub fn new(fs: ProjectFs) -> Self {
        Self {
            fs,
            textures: HashMap::new(),
            paths: HashMap::new(),
            texture_bytes: 0,
        }
    }

    pub fn load_texture(&mut self, relative: &str) -> Result<TextureHandle> {
        let path = self.fs.resolve(relative)?;
        if let Some(handle) = self.paths.get(&path) {
            return Ok(*handle);
        }
        ensure!(
            self.textures.len() < 4096,
            "texture count limit reached (4096)"
        );
        let asset = decode_texture(&self.fs.read(relative)?)
            .with_context(|| format!("cannot decode texture {relative}"))?;
        let (width, height) = (asset.width, asset.height);
        ensure!(
            self.texture_bytes + asset.rgba.len() <= MAX_CACHE_BYTES,
            "texture cache exceeds 256 MiB"
        );
        let handle = TextureHandle::allocate()?;
        self.texture_bytes += asset.rgba.len();
        self.textures.insert(handle, asset);
        self.paths.insert(path, handle);
        log::info!("Loaded texture {relative} ({width}x{height})");
        Ok(handle)
    }

    pub fn texture(&self, handle: TextureHandle) -> Result<&TextureAsset> {
        self.textures
            .get(&handle)
            .context("unknown texture handle (resource may belong to an old session)")
    }

    pub fn contains(&self, handle: TextureHandle) -> bool {
        self.textures.contains_key(&handle)
    }

    pub fn release(&mut self, handle: TextureHandle) -> Result<()> {
        let asset = self
            .textures
            .remove(&handle)
            .context("unknown or released texture")?;
        self.texture_bytes -= asset.rgba.len();
        self.paths.retain(|_, cached| *cached != handle);
        Ok(())
    }

    pub fn set_filter(&mut self, handle: TextureHandle, filter: TextureFilter) -> Result<()> {
        let asset = self
            .textures
            .get_mut(&handle)
            .context("unknown or released texture")?;
        if asset.filter != filter {
            asset.revision = asset
                .revision
                .checked_add(1)
                .context("texture revision exhausted")?;
            asset.filter = filter;
        }
        Ok(())
    }

    /// Decode before replacing; callers keep the old pixels on any failure.
    pub fn reload_texture(&mut self, relative: &Path) -> Result<bool> {
        let path = self.fs.resolve(relative)?;
        let Some(&handle) = self.paths.get(&path) else {
            return Ok(false);
        };
        let mut replacement = decode_texture(&self.fs.read(relative)?)?;
        let previous = self.texture(handle)?;
        ensure!(
            (replacement.width, replacement.height) == (previous.width, previous.height),
            "texture size changed; press F5 to restart the game with the new dimensions"
        );
        replacement.filter = previous.filter;
        replacement.revision = previous
            .revision
            .checked_add(1)
            .context("texture revision exhausted")?;
        self.textures.insert(handle, replacement);
        Ok(true)
    }

    /// Allocate an engine-owned texture, used by glyph atlases and generated assets.
    pub fn insert_pixels(
        &mut self,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
        filter: TextureFilter,
    ) -> Result<TextureHandle> {
        ensure!(
            width > 0 && height > 0 && width <= 8192 && height <= 8192,
            "invalid texture size"
        );
        ensure!(
            rgba.len() as u64 == u64::from(width) * u64::from(height) * 4,
            "invalid texture byte count"
        );
        ensure!(
            rgba.len() <= MAX_TEXTURE_BYTES && self.texture_bytes + rgba.len() <= MAX_CACHE_BYTES,
            "texture memory budget exceeded"
        );
        ensure!(self.textures.len() < 4096, "texture count limit exceeded");
        let handle = TextureHandle::allocate()?;
        self.texture_bytes += rgba.len();
        self.textures.insert(
            handle,
            TextureAsset {
                width,
                height,
                rgba,
                filter,
                revision: 1,
            },
        );
        Ok(handle)
    }

    pub fn write_region(
        &mut self,
        handle: TextureHandle,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        let texture = self.textures.get_mut(&handle).context("unknown texture")?;
        ensure!(
            u64::from(x) + u64::from(width) <= u64::from(texture.width)
                && u64::from(y) + u64::from(height) <= u64::from(texture.height),
            "region exceeds texture bounds"
        );
        ensure!(
            rgba.len() as u64 == u64::from(width) * u64::from(height) * 4,
            "invalid region byte count"
        );
        let revision = texture
            .revision
            .checked_add(1)
            .context("texture revision exhausted")?;
        let stride = width as usize * 4;
        for row in 0..height as usize {
            let offset = ((y as usize + row) * texture.width as usize + x as usize) * 4;
            texture.rgba[offset..offset + stride]
                .copy_from_slice(&rgba[row * stride..(row + 1) * stride]);
        }
        texture.revision = revision;
        Ok(())
    }

    pub fn texture_count(&self) -> usize {
        self.textures.len()
    }

    pub fn texture_bytes(&self) -> usize {
        self.texture_bytes
    }
}

fn decode_texture(bytes: &[u8]) -> Result<TextureAsset> {
    let reader = image::io::Reader::new(Cursor::new(bytes)).with_guessed_format()?;
    let (width, height) = reader
        .into_dimensions()
        .context("cannot read image dimensions")?;
    let decoded_bytes = u64::from(width) * u64::from(height) * 4;
    ensure!(
        width > 0 && height > 0 && width <= 8192 && height <= 8192,
        "texture dimensions must be in 1..=8192"
    );
    ensure!(
        decoded_bytes <= MAX_TEXTURE_BYTES as u64,
        "texture exceeds 64 MiB decoded limit"
    );
    let image = image::load_from_memory(bytes)?.to_rgba8();
    ensure!(
        image.dimensions() == (width, height),
        "inconsistent image dimensions"
    );
    Ok(TextureAsset {
        width,
        height,
        rgba: image.into_raw(),
        filter: TextureFilter::Nearest,
        revision: 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (tempfile::TempDir, AssetManager) {
        let root = tempfile::tempdir().unwrap();
        image::RgbaImage::from_pixel(2, 3, image::Rgba([255, 0, 0, 255]))
            .save(root.path().join("test.png"))
            .unwrap();
        let manager = AssetManager::new(ProjectFs::new(root.path()).unwrap());
        (root, manager)
    }

    #[test]
    fn equivalent_paths_share_a_single_texture() {
        let (_root, mut assets) = fixture();
        let a = assets.load_texture("test.png").unwrap();
        let b = assets.load_texture("./test.png").unwrap();
        assert_eq!(a, b);
        assert_eq!(assets.texture_count(), 1);
        assert_eq!(assets.texture_bytes(), 24);
        assert_eq!(assets.texture(a).unwrap().height, 3);
    }

    #[test]
    fn failed_loads_do_not_poison_the_cache() {
        let (root, mut assets) = fixture();
        std::fs::write(root.path().join("bad.png"), "not an image").unwrap();
        assert!(assets.load_texture("bad.png").is_err());
        assert!(assets.load_texture("missing.png").is_err());
        assert_eq!(assets.texture_count(), 0);
        assert_eq!(assets.texture_bytes(), 0);
    }

    #[test]
    fn fresh_managers_cannot_resolve_old_handles() {
        let (root, mut assets) = fixture();
        let old = assets.load_texture("test.png").unwrap();
        let mut new_assets = AssetManager::new(ProjectFs::new(root.path()).unwrap());
        new_assets.load_texture("test.png").unwrap();
        assert!(new_assets.texture(old).is_err());
    }

    #[test]
    fn released_handles_never_alias_reloaded_assets() {
        let (_root, mut assets) = fixture();
        let old = assets.load_texture("test.png").unwrap();
        assets.release(old).unwrap();
        assert_eq!(assets.texture_bytes(), 0);
        let fresh = assets.load_texture("test.png").unwrap();
        assert_ne!(old, fresh);
        assert!(assets.texture(old).is_err());
        assert!(assets.release(old).is_err());
    }

    #[test]
    fn reload_is_atomic_and_preserves_handles_and_filtering() {
        let (root, mut assets) = fixture();
        let handle = assets.load_texture("test.png").unwrap();
        assets.set_filter(handle, TextureFilter::Linear).unwrap();
        std::fs::write(root.path().join("test.png"), "broken").unwrap();
        assert!(assets.reload_texture(Path::new("test.png")).is_err());
        assert_eq!(assets.texture(handle).unwrap().rgba[0], 255);
        image::RgbaImage::from_pixel(2, 3, image::Rgba([0, 255, 0, 255]))
            .save(root.path().join("test.png"))
            .unwrap();
        assert!(assets.reload_texture(Path::new("test.png")).unwrap());
        let new = assets.texture(handle).unwrap();
        assert_eq!(&new.rgba[..4], &[0, 255, 0, 255]);
        assert_eq!(new.filter, TextureFilter::Linear);
        assert_eq!(new.revision, 3);
        assert_eq!(assets.texture_bytes(), 24);
        image::RgbaImage::new(3, 3)
            .save(root.path().join("test.png"))
            .unwrap();
        assert!(assets.reload_texture(Path::new("test.png")).is_err());
        assert_eq!(assets.texture(handle).unwrap().width, 2);
    }
}
