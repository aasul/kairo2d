use anyhow::{ensure, Result};
use bytemuck::{Pod, Zeroable};
use font8x8::UnicodeFonts;
use glam::Vec2;
use kairo_core::{DrawCommand, Frame, PixelRect, Quad, TextureHandle, Transform};
use std::ops::Range;

const MAX_VERTICES: usize = 1_500_000;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Debug)]
pub struct Batch {
    pub texture: Option<TextureHandle>,
    pub vertices: Range<u32>,
    pub viewport: PixelRect,
    pub scissor: Option<PixelRect>,
}

#[derive(Default)]
pub struct Mesh {
    pub pixel_snap: bool,
    pub vertices: Vec<Vertex>,
    pub batches: Vec<Batch>,
}

impl Mesh {
    pub fn build(&mut self, frame: &Frame, width: u32, height: u32) -> Result<()> {
        ensure!(width > 0 && height > 0, "render viewport must be nonzero");
        self.vertices.clear();
        self.batches.clear();
        let full = PixelRect::full(width, height);
        let mut viewport = full;
        let mut scissor = None;
        for command in &frame.commands {
            match command {
                DrawCommand::Viewport(rect) => {
                    viewport = rect.unwrap_or(full);
                    ensure!(
                        viewport.width > 0
                            && viewport.height > 0
                            && viewport.intersection(full) == Some(viewport),
                        "viewport must fit inside the window"
                    );
                }
                DrawCommand::Scissor(rect) => scissor = *rect,
                DrawCommand::Quad(quad) => self.quad(quad, viewport, scissor)?,
                DrawCommand::Text {
                    text,
                    position,
                    scale,
                    camera,
                    color,
                } => {
                    let mut cursor = *position;
                    for character in text.chars() {
                        match character {
                            '\n' => {
                                cursor.x = position.x;
                                cursor.y += 10.0 * scale;
                                continue;
                            }
                            '\t' => {
                                cursor.x += 32.0 * scale;
                                continue;
                            }
                            '\r' => continue,
                            _ => {}
                        }
                        let glyph = font8x8::BASIC_FONTS
                            .get(character)
                            .or_else(|| font8x8::BASIC_FONTS.get('?'))
                            .unwrap_or([0; 8]);
                        for (row, bits) in glyph.iter().enumerate() {
                            let mut column = 0;
                            while column < 8 {
                                if bits & (1 << column) == 0 {
                                    column += 1;
                                    continue;
                                }
                                let start = column;
                                while column < 8 && bits & (1 << column) != 0 {
                                    column += 1;
                                }
                                self.quad(
                                    &Quad {
                                        texture: None,
                                        size: Vec2::new((column - start) as f32 * scale, *scale),
                                        uv_min: Vec2::ZERO,
                                        uv_max: Vec2::ONE,
                                        transform: Transform {
                                            position: cursor
                                                + Vec2::new(
                                                    start as f32 * scale,
                                                    row as f32 * scale,
                                                ),
                                            ..Default::default()
                                        },
                                        camera: *camera,
                                        color: *color,
                                    },
                                    viewport,
                                    scissor,
                                )?;
                            }
                        }
                        cursor.x += 8.0 * scale;
                    }
                }
            }
        }
        Ok(())
    }

    fn quad(&mut self, quad: &Quad, viewport: PixelRect, scissor: Option<PixelRect>) -> Result<()> {
        if scissor.is_some_and(|clip| clip.intersection(viewport).is_none()) {
            return Ok(());
        }
        ensure!(
            self.vertices.len() + 6 <= MAX_VERTICES,
            "frame exceeds geometry limit (250000 quads)"
        );
        let local = [
            Vec2::ZERO,
            Vec2::new(quad.size.x, 0.0),
            quad.size,
            Vec2::new(0.0, quad.size.y),
        ];
        let uv = [
            quad.uv_min,
            Vec2::new(quad.uv_max.x, quad.uv_min.y),
            quad.uv_max,
            Vec2::new(quad.uv_min.x, quad.uv_max.y),
        ];
        let color = quad.color.linear();
        let start = self.vertices.len() as u32;
        for i in [0, 1, 2, 0, 2, 3] {
            let p = quad.camera.world_to_screen(quad.transform.point(local[i]));
            let p = if self.pixel_snap { p.round() } else { p };
            ensure!(
                p.is_finite(),
                "draw transform produced non-finite coordinates"
            );
            self.vertices.push(Vertex {
                position: [
                    p.x / viewport.width as f32 * 2.0 - 1.0,
                    1.0 - p.y / viewport.height as f32 * 2.0,
                ],
                uv: uv[i].to_array(),
                color,
            });
        }
        let end = self.vertices.len() as u32;
        // Only adjacent commands may merge: globally sorting transparent sprites
        // would change the painter's order and therefore the final image.
        if let Some(batch) = self.batches.last_mut() {
            if batch.texture == quad.texture
                && batch.viewport == viewport
                && batch.scissor == scissor
            {
                batch.vertices.end = end;
                return Ok(());
            }
        }
        self.batches.push(Batch {
            texture: quad.texture,
            vertices: start..end,
            viewport,
            scissor,
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kairo_core::{Camera, Color};

    fn quad(texture: Option<TextureHandle>) -> DrawCommand {
        DrawCommand::Quad(Quad {
            texture,
            size: Vec2::splat(10.0),
            uv_min: Vec2::ZERO,
            uv_max: Vec2::ONE,
            transform: Transform::default(),
            camera: Camera::default(),
            color: Color::WHITE,
        })
    }

    #[test]
    fn adjacent_textures_batch_without_reordering() {
        let a = TextureHandle::allocate().unwrap();
        let b = TextureHandle::allocate().unwrap();
        let frame = Frame {
            commands: vec![quad(Some(a)), quad(Some(a)), quad(Some(b)), quad(Some(a))],
            ..Default::default()
        };
        let mut mesh = Mesh::default();
        mesh.build(&frame, 100, 100).unwrap();
        assert_eq!(mesh.batches.len(), 3);
        assert_eq!(mesh.batches[0].vertices, 0..12);
        assert_eq!(mesh.batches[2].texture, Some(a));
        assert_eq!(mesh.vertices.len(), 24);
    }

    #[test]
    fn top_left_pixel_maps_to_top_left_clip_space() {
        let mut mesh = Mesh::default();
        let frame = Frame {
            commands: vec![quad(None)],
            ..Default::default()
        };
        mesh.build(&frame, 100, 100).unwrap();
        assert_eq!(mesh.vertices[0].position, [-1.0, 1.0]);
        assert_eq!(mesh.vertices[2].position, [-0.8, 0.8]);
    }

    #[test]
    fn bitmap_text_uses_the_solid_batch() {
        let frame = Frame {
            commands: vec![DrawCommand::Text {
                text: "Kairo2D".into(),
                position: Vec2::ZERO,
                scale: 2.0,
                camera: Camera::default(),
                color: Color::WHITE,
            }],
            ..Default::default()
        };
        let mut mesh = Mesh::default();
        mesh.build(&frame, 640, 480).unwrap();
        assert!(!mesh.vertices.is_empty());
        assert_eq!(mesh.batches.len(), 1);
        assert_eq!(mesh.batches[0].texture, None);
    }

    #[test]
    fn viewport_and_scissor_changes_split_batches() {
        let view = PixelRect {
            x: 40,
            y: 0,
            width: 50,
            height: 50,
        };
        let clip = PixelRect {
            x: 40,
            y: 0,
            width: 10,
            height: 10,
        };
        let frame = Frame {
            commands: vec![
                quad(None),
                DrawCommand::Viewport(Some(view)),
                quad(None),
                DrawCommand::Scissor(Some(clip)),
                quad(None),
            ],
            ..Default::default()
        };
        let mut mesh = Mesh::default();
        mesh.build(&frame, 100, 100).unwrap();
        assert_eq!(mesh.batches.len(), 3);
        assert_eq!(mesh.batches[1].viewport, view);
        assert_eq!(mesh.batches[2].scissor, Some(clip));
        assert_eq!(mesh.vertices[6].position, [-1.0, 1.0]);
    }

    #[test]
    fn empty_clip_drops_geometry_and_outside_viewport_is_an_error() {
        let empty = PixelRect {
            x: 0,
            y: 0,
            width: 0,
            height: 20,
        };
        let mut frame = Frame {
            commands: vec![DrawCommand::Scissor(Some(empty)), quad(None)],
            ..Default::default()
        };
        let mut mesh = Mesh::default();
        mesh.build(&frame, 100, 100).unwrap();
        assert!(mesh.vertices.is_empty());
        frame.commands = vec![DrawCommand::Viewport(Some(PixelRect::full(200, 100)))];
        assert!(mesh.build(&frame, 100, 100).is_err());
    }
}
