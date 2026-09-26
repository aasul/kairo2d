use crate::TextureHandle;
use anyhow::{ensure, Result};
use glam::{Mat2, Mat3, Vec2};

pub const MAX_COMMANDS: usize = 50_000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color(pub [f32; 4]);

impl Color {
    pub const WHITE: Self = Self([1.0; 4]);
    pub const BACKGROUND: Self = Self([0.055, 0.067, 0.09, 1.0]);

    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Result<Self> {
        let channels = [r, g, b, a];
        ensure!(
            channels
                .iter()
                .all(|c| c.is_finite() && (0.0..=1.0).contains(c)),
            "color components must be finite numbers in 0..=1"
        );
        Ok(Self(channels))
    }

    pub fn linear(self) -> [f32; 4] {
        let convert = |c: f32| {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        [
            convert(self.0[0]),
            convert(self.0[1]),
            convert(self.0[2]),
            self.0[3],
        ]
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub position: Vec2,
    pub zoom: f32,
    pub rotation: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            zoom: 1.0,
            rotation: 0.0,
        }
    }
}

impl Camera {
    pub fn world_to_screen(self, position: Vec2) -> Vec2 {
        Mat2::from_angle(-self.rotation) * (position - self.position) * self.zoom
    }

    pub fn screen_to_world(self, position: Vec2) -> Vec2 {
        Mat2::from_angle(self.rotation) * (position / self.zoom) + self.position
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub position: Vec2,
    pub scale: Vec2,
    pub origin: Vec2,
    pub rotation: f32,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            scale: Vec2::ONE,
            origin: Vec2::ZERO,
            rotation: 0.0,
        }
    }
}

impl Transform {
    pub fn point(self, local: Vec2) -> Vec2 {
        self.position + Mat2::from_angle(self.rotation) * ((local - self.origin) * self.scale)
    }
}

#[derive(Clone, Debug)]
pub struct Quad {
    pub texture: Option<TextureHandle>,
    pub size: Vec2,
    pub uv_min: Vec2,
    pub uv_max: Vec2,
    pub transform: Transform,
    pub camera: Camera,
    pub color: Color,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl PixelRect {
    pub fn full(width: u32, height: u32) -> Self {
        Self {
            x: 0,
            y: 0,
            width,
            height,
        }
    }

    pub fn intersection(self, other: Self) -> Option<Self> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self
            .x
            .saturating_add(self.width)
            .min(other.x.saturating_add(other.width));
        let bottom = self
            .y
            .saturating_add(self.height)
            .min(other.y.saturating_add(other.height));
        (right > x && bottom > y).then_some(Self {
            x,
            y,
            width: right.saturating_sub(x),
            height: bottom.saturating_sub(y),
        })
    }
}

#[derive(Clone, Debug)]
pub enum DrawCommand {
    Viewport(Option<PixelRect>),
    Scissor(Option<PixelRect>),
    Quad(Quad),
    /// A scene quad with an exact inherited affine transform. Keeps shear from
    /// rotated children under non-uniformly scaled parents.
    AffineQuad {
        quad: Quad,
        matrix: Mat3,
    },
    Text {
        text: String,
        position: Vec2,
        scale: f32,
        camera: Camera,
        color: Color,
    },
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub clear: Color,
    pub commands: Vec<DrawCommand>,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            clear: Color::BACKGROUND,
            commands: Vec::new(),
        }
    }
}

impl Frame {
    pub fn reset(&mut self) {
        self.clear = Color::BACKGROUND;
        self.commands.clear();
    }

    pub fn push(&mut self, command: DrawCommand) -> Result<()> {
        ensure!(
            self.commands.len() < MAX_COMMANDS,
            "frame exceeds {MAX_COMMANDS} draw commands"
        );
        self.commands.push(command);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_coordinates_round_trip() {
        let camera = Camera {
            position: Vec2::new(40.0, 70.0),
            zoom: 2.5,
            rotation: 0.7,
        };
        let point = Vec2::new(91.0, -13.0);
        assert!(
            camera
                .screen_to_world(camera.world_to_screen(point))
                .distance(point)
                < 0.001
        );
    }

    #[test]
    fn origin_is_applied_before_scale_and_rotation() {
        let transform = Transform {
            position: Vec2::new(100.0, 200.0),
            origin: Vec2::splat(16.0),
            scale: Vec2::splat(2.0),
            rotation: std::f32::consts::FRAC_PI_2,
        };
        assert!(
            transform
                .point(Vec2::splat(16.0))
                .distance(transform.position)
                < 0.001
        );
        assert!(
            transform
                .point(Vec2::new(17.0, 16.0))
                .distance(Vec2::new(100.0, 202.0))
                < 0.001
        );
    }

    #[test]
    fn color_validation_and_linear_conversion() {
        assert!(Color::new(f32::NAN, 0.0, 0.0, 1.0).is_err());
        assert!(Color::new(255.0, 0.0, 0.0, 1.0).is_err());
        let linear = Color::new(0.5, 0.5, 0.5, 0.3).unwrap().linear();
        assert!((linear[0] - 0.214041).abs() < 0.00001);
        assert_eq!(linear[3], 0.3);
    }
}
