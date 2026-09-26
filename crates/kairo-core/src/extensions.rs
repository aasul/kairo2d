//! Statically compiled Rust extensions hosted by the engine.
use crate::{Camera, Color, DrawCommand, Frame, InputState, Quad, TextureHandle, Transform};
use anyhow::{ensure, Result};
use glam::Vec2;

pub trait NativeExtension {
    fn name(&self) -> &str;
    fn update(&mut self, dt: f32, input: &InputState) -> Result<()>;
    fn draw(&mut self, canvas: &mut Canvas<'_>) -> Result<()>;
}

pub struct Canvas<'a> {
    frame: &'a mut Frame,
    size: [u32; 2],
}
impl<'a> Canvas<'a> {
    pub fn new(frame: &'a mut Frame, size: [u32; 2]) -> Self {
        Self { frame, size }
    }
    pub fn size(&self) -> [u32; 2] {
        self.size
    }
    pub fn rectangle(&mut self, position: [f32; 2], size: [f32; 2], color: Color) -> Result<()> {
        crate::finite(
            "native rectangle",
            &[position[0], position[1], size[0], size[1]],
        )?;
        ensure!(
            size[0] >= 0.0 && size[1] >= 0.0,
            "negative native rectangle size"
        );
        self.quad(None, position, size, color)
    }
    pub fn sprite(
        &mut self,
        texture: TextureHandle,
        position: [f32; 2],
        size: [f32; 2],
        tint: Color,
    ) -> Result<()> {
        crate::finite(
            "native sprite",
            &[position[0], position[1], size[0], size[1]],
        )?;
        ensure!(
            size[0] >= 0.0 && size[1] >= 0.0,
            "negative native sprite size"
        );
        self.quad(Some(texture), position, size, tint)
    }
    fn quad(
        &mut self,
        texture: Option<TextureHandle>,
        position: [f32; 2],
        size: [f32; 2],
        color: Color,
    ) -> Result<()> {
        crate::finite("native color", &color.0)?;
        ensure!(
            color.0.iter().all(|v| (0.0..=1.0).contains(v)),
            "native colors must be normalized"
        );
        self.frame.push(DrawCommand::Quad(Quad {
            texture,
            size: size.into(),
            uv_min: Vec2::ZERO,
            uv_max: Vec2::ONE,
            transform: Transform {
                position: position.into(),
                ..Default::default()
            },
            camera: Camera::default(),
            color,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extension_canvas_emits_validated_engine_commands() {
        let mut frame = Frame::default();
        let mut canvas = Canvas::new(&mut frame, [320, 180]);
        assert_eq!(canvas.size(), [320, 180]);
        assert!(canvas
            .rectangle([0.0; 2], [-1.0, 10.0], Color::WHITE)
            .is_err());
        canvas
            .rectangle([4.0, 8.0], [16.0, 16.0], Color::WHITE)
            .unwrap();
        assert_eq!(frame.commands.len(), 1);
    }
}
