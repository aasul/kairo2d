use crate::{Color, PixelRect};
use anyhow::{bail, Result};

/// Center a virtual canvas; use fractional downscaling only when the window is too small.
pub fn viewport(screen: [u32; 2], canvas: [u32; 2], integer: bool) -> PixelRect {
    if screen.contains(&0) || canvas.contains(&0) {
        return PixelRect::full(0, 0);
    }
    let scale = (screen[0] as f64 / canvas[0] as f64).min(screen[1] as f64 / canvas[1] as f64);
    let scale = if integer && scale >= 1.0 {
        scale.floor()
    } else {
        scale
    };
    let width = (canvas[0] as f64 * scale).round().max(1.0) as u32;
    let height = (canvas[1] as f64 * scale).round().max(1.0) as u32;
    PixelRect {
        x: screen[0].saturating_sub(width) / 2,
        y: screen[1].saturating_sub(height) / 2,
        width: width.min(screen[0]),
        height: height.min(screen[1]),
    }
}

pub fn pointer(position: [f32; 2], viewport: PixelRect, canvas: [u32; 2]) -> [f32; 2] {
    [
        (position[0] - viewport.x as f32) * canvas[0] as f32 / viewport.width.max(1) as f32,
        (position[1] - viewport.y as f32) * canvas[1] as f32 / viewport.height.max(1) as f32,
    ]
}

pub fn palette(name: &str) -> Result<Vec<Color>> {
    let values: &[u32] = match name {
        "none" => &[],
        "kairo16" => &[
            0x121822, 0x29314a, 0x4c4265, 0x785878, 0xb57183, 0xe7a58a, 0xf4d7a1, 0xf8f3df,
            0x24423c, 0x397057, 0x68a56b, 0xb3d47b, 0x263f68, 0x3b6e96, 0x69afbc, 0xb9dce1,
        ],
        "grayscale" => &[0x151515, 0x626262, 0xababab, 0xf0f0f0],
        "olive4" => &[0x1c3028, 0x4c6540, 0x91a65f, 0xd6dfa2],
        _ => bail!("unknown Micro palette; use kairo16, grayscale, olive4, or none"),
    };
    Ok(values
        .iter()
        .map(|color| {
            Color([
                ((color >> 16) & 255) as f32 / 255.0,
                ((color >> 8) & 255) as f32 / 255.0,
                (color & 255) as f32 / 255.0,
                1.0,
            ])
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn integer_scaling_centers_letterbox_and_pointer_matches_canvas() {
        let rect = viewport([1000, 700], [320, 180], true);
        assert_eq!(
            rect,
            PixelRect {
                x: 20,
                y: 80,
                width: 960,
                height: 540
            }
        );
        assert_eq!(pointer([500.0, 350.0], rect, [320, 180]), [160.0, 90.0]);
        assert_eq!(
            viewport([160, 90], [320, 180], true),
            PixelRect::full(160, 90)
        );
    }
    #[test]
    fn palettes_have_bounded_valid_colors() {
        for name in ["kairo16", "grayscale", "olive4", "none"] {
            let colors = palette(name).unwrap();
            assert!(colors.len() <= 16);
            assert!(colors
                .iter()
                .all(|color| color.0.iter().all(|channel| (0.0..=1.0).contains(channel))));
        }
        assert!(palette("typo").is_err());
    }
}
