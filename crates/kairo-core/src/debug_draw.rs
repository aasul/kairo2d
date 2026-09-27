use crate::{Camera, Color, DrawCommand, Frame, Quad, Transform};
use anyhow::Result;
use glam::Vec2;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct DebugDraw {
    pub physics: bool,
    pub bounds: bool,
    pub velocities: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct DebugSegment {
    pub from: Vec2,
    pub to: Vec2,
    pub color: Color,
}
impl DebugSegment {
    pub fn draw(self, frame: &mut Frame, camera: Camera) -> Result<()> {
        let delta = self.to - self.from;
        if !self.from.is_finite() || !delta.is_finite() || delta.length_squared() < 0.00001 {
            return Ok(());
        }
        frame.push(DrawCommand::Quad(Quad {
            texture: None,
            size: Vec2::new(delta.length(), 1.0 / camera.zoom.max(0.001)),
            uv_min: Vec2::ZERO,
            uv_max: Vec2::ONE,
            transform: Transform {
                position: self.from,
                rotation: delta.y.atan2(delta.x),
                ..Default::default()
            },
            camera,
            color: self.color,
        }))
    }
}
/// Bound the overlay even when a game submits thousands of sprites.
pub fn sprite_bounds(frame: &mut Frame) -> Result<()> {
    let mut source = Vec::new();
    let mut viewport = None;
    let mut scissor = None;
    for command in &frame.commands {
        match command {
            DrawCommand::Viewport(value) => viewport = *value,
            DrawCommand::Scissor(value) => scissor = *value,
            DrawCommand::Quad(quad) if quad.texture.is_some() && source.len() < 1024 => {
                source.push((viewport, scissor, quad.clone(), None));
            }
            DrawCommand::AffineQuad { quad, matrix }
                if quad.texture.is_some() && source.len() < 1024 =>
            {
                source.push((viewport, scissor, quad.clone(), Some(*matrix)));
            }
            _ => {}
        }
    }
    for (view, clip, quad, matrix) in source {
        frame.push(DrawCommand::Viewport(view))?;
        frame.push(DrawCommand::Scissor(clip))?;
        let p = [
            Vec2::ZERO,
            Vec2::new(quad.size.x, 0.0),
            quad.size,
            Vec2::new(0.0, quad.size.y),
        ];
        for i in 0..4 {
            DebugSegment {
                from: matrix
                    .map_or_else(|| quad.transform.point(p[i]), |m| m.transform_point2(p[i])),
                to: matrix.map_or_else(
                    || quad.transform.point(p[(i + 1) % 4]),
                    |m| m.transform_point2(p[(i + 1) % 4]),
                ),
                color: Color([1.0, 0.7, 0.2, 0.85]),
            }
            .draw(frame, quad.camera)?;
        }
    }
    frame.push(DrawCommand::Viewport(viewport))?;
    frame.push(DrawCommand::Scissor(scissor))?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn debug_lines_use_the_normal_queue_and_zero_length_is_skipped() {
        let mut frame = Frame::default();
        DebugSegment {
            from: Vec2::ZERO,
            to: Vec2::ZERO,
            color: Color::WHITE,
        }
        .draw(&mut frame, Camera::default())
        .unwrap();
        assert!(frame.commands.is_empty());
        DebugSegment {
            from: Vec2::ZERO,
            to: Vec2::X * 10.0,
            color: Color::WHITE,
        }
        .draw(&mut frame, Camera::default())
        .unwrap();
        assert_eq!(frame.commands.len(), 1);
    }
}
