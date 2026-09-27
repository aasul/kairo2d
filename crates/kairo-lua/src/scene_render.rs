//! Converts authored scene nodes to the existing ordered native draw queue.
//! No GPU or asset handles are stored in the serialized scene resource.
use crate::state::EngineState;
use anyhow::{ensure, Context, Result};
use glam::{Mat2, Mat3, Vec2};
use kairo_core::debug_draw::DebugSegment;
use kairo_core::scene_graph::{NodeFile, NodeId, NodeKind, SceneGraph};
use kairo_core::{Camera, Color, DrawCommand, Quad, Transform};
use serde_json::Value;

fn number(node: &NodeFile, key: &str, default: f32) -> Result<f32> {
    match node.properties.get(key) {
        None => Ok(default),
        Some(Value::Number(n)) => {
            let value = n.as_f64().context("number cannot be represented")? as f32;
            ensure!(value.is_finite(), "{key} must be finite");
            Ok(value)
        }
        _ => anyhow::bail!("{key} must be a number"),
    }
}

fn boolean(node: &NodeFile, key: &str, default: bool) -> Result<bool> {
    match node.properties.get(key) {
        None => Ok(default),
        Some(Value::Bool(value)) => Ok(*value),
        _ => anyhow::bail!("{key} must be a boolean"),
    }
}

fn string<'a>(node: &'a NodeFile, key: &str) -> Result<Option<&'a str>> {
    match node.properties.get(key) {
        None => Ok(None),
        Some(Value::String(value)) => Ok(Some(value)),
        _ => anyhow::bail!("{key} must be a string"),
    }
}

fn color(node: &NodeFile, key: &str, default: Color) -> Result<Color> {
    let Some(value) = node.properties.get(key) else {
        return Ok(default);
    };
    let channels = value
        .as_array()
        .context("color must be an RGB or RGBA array")?;
    ensure!(
        channels.len() == 3 || channels.len() == 4,
        "color must have three or four channels"
    );
    let channel = |i: usize| -> Result<f32> {
        Ok(channels[i]
            .as_f64()
            .context("color channel must be a number")? as f32)
    };
    Color::new(
        channel(0)?,
        channel(1)?,
        channel(2)?,
        if channels.len() == 4 {
            channel(3)?
        } else {
            1.0
        },
    )
}

fn source(node: &NodeFile, width: f32, height: f32) -> Result<[f32; 4]> {
    let Some(value) = node.properties.get("source") else {
        return Ok([0.0, 0.0, width, height]);
    };
    let object = value.as_object().context("source must be an object")?;
    let read = |key: &str| -> Result<f32> {
        let value = object
            .get(key)
            .context(format!("source.{key} is required"))?;
        let number = value
            .as_f64()
            .context(format!("source.{key} must be a number"))? as f32;
        ensure!(number.is_finite(), "source.{key} must be finite");
        Ok(number)
    };
    Ok([read("x")?, read("y")?, read("width")?, read("height")?])
}

fn active_camera(graph: &mut SceneGraph, size: Vec2) -> Result<Option<Camera>> {
    let mut pending = vec![graph.root()];
    while let Some(id) = pending.pop() {
        let node = graph.node(id)?;
        if !node.data().enabled {
            continue;
        }
        let kind = node.data().kind;
        let properties = node.data().clone();
        pending.extend(node.children().iter().rev());
        if kind != NodeKind::Camera2D || !boolean(&properties, "active", false)? {
            continue;
        }
        let zoom = number(&properties, "zoom", 1.0)?;
        ensure!(zoom > 0.0, "active camera zoom must be positive");
        let world = graph.world_matrix(id)?;
        let center = graph.world_position(id)?;
        let axis = world.x_axis.truncate();
        let rotation = axis.y.atan2(axis.x);
        let offset = Mat2::from_angle(rotation) * (size / (2.0 * zoom));
        return Ok(Some(Camera {
            position: center - offset,
            zoom,
            rotation,
        }));
    }
    Ok(None)
}

fn draw_node(node: &NodeFile, matrix: Mat3, camera: Camera, state: &mut EngineState) -> Result<()> {
    match node.kind {
        NodeKind::Sprite | NodeKind::AnimatedSprite => {
            let Some(path) = string(node, "texture")? else {
                return Ok(());
            };
            let texture = state.assets.load_texture(path)?;
            let asset = state.assets.texture(texture)?;
            let tw = asset.width as f32;
            let th = asset.height as f32;
            let [x, y, w, h] = source(node, tw, th)?;
            ensure!(
                x >= 0.0 && y >= 0.0 && w > 0.0 && h > 0.0 && x + w <= tw && y + h <= th,
                "sprite source is outside texture"
            );
            let width = number(node, "width", w)?;
            let height = number(node, "height", h)?;
            ensure!(
                width > 0.0 && height > 0.0,
                "sprite dimensions must be positive"
            );
            let mut uv_min = Vec2::new(x / tw, y / th);
            let mut uv_max = Vec2::new((x + w) / tw, (y + h) / th);
            if boolean(node, "flip_x", false)? {
                std::mem::swap(&mut uv_min.x, &mut uv_max.x);
            }
            if boolean(node, "flip_y", false)? {
                std::mem::swap(&mut uv_min.y, &mut uv_max.y);
            }
            state.frame.push(DrawCommand::AffineQuad {
                quad: Quad {
                    texture: Some(texture),
                    size: Vec2::new(width, height),
                    uv_min,
                    uv_max,
                    transform: Transform::default(),
                    camera,
                    color: color(node, "tint", Color::WHITE)?,
                },
                matrix,
            })?;
        }
        NodeKind::Control => {
            let width = number(node, "width", 0.0)?;
            let height = number(node, "height", 0.0)?;
            ensure!(
                width >= 0.0 && height >= 0.0,
                "control dimensions cannot be negative"
            );
            if width > 0.0 && height > 0.0 {
                state.frame.push(DrawCommand::AffineQuad {
                    quad: Quad {
                        texture: None,
                        size: Vec2::new(width, height),
                        uv_min: Vec2::ZERO,
                        uv_max: Vec2::ONE,
                        transform: Transform::default(),
                        camera,
                        color: color(node, "color", Color::WHITE)?,
                    },
                    matrix,
                })?;
            }
        }
        NodeKind::Text => {
            if let Some(text) = string(node, "text")? {
                ensure!(text.len() <= 4096, "scene text exceeds 4096 bytes");
                let scale = number(node, "scale", 2.0)?;
                ensure!(scale > 0.0, "text scale must be positive");
                state.frame.push(DrawCommand::Text {
                    text: text.to_owned(),
                    position: matrix.transform_point2(Vec2::ZERO),
                    scale,
                    camera,
                    color: color(node, "color", Color::WHITE)?,
                })?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn draw(graph: &mut SceneGraph, state: &mut EngineState) -> Result<()> {
    state.require_draw()?;
    let size = Vec2::new(state.width as f32, state.height as f32);
    if let Some(camera) = active_camera(graph, size)? {
        state.camera = camera;
    }
    let camera = state.camera;
    let mut debug_nodes = 0;
    let mut pending: Vec<(NodeId, bool)> = vec![(graph.root(), false)];
    while let Some((id, inherited_screen)) = pending.pop() {
        let node = graph.node(id)?;
        if !node.data().enabled || !node.data().visible {
            continue;
        }
        let data = node.data().clone();
        let screen = inherited_screen || data.kind == NodeKind::CanvasLayer;
        pending.extend(node.children().iter().rev().map(|child| (*child, screen)));
        let matrix = graph.world_matrix(id)?;
        let node_camera = if screen { Camera::default() } else { camera };
        draw_node(&data, matrix, node_camera, state).with_context(|| {
            format!(
                "scene '{}', node '{}': native draw",
                graph.name(),
                graph.path(id).unwrap_or_default()
            )
        })?;
        if state.development_tools
            && debug_nodes < 256
            && (state.debug_draw.origins || state.debug_draw.names)
        {
            let origin = matrix.transform_point2(Vec2::ZERO);
            if state.debug_draw.origins {
                let extent = 5.0 / node_camera.zoom.max(0.001);
                for axis in [Vec2::X, Vec2::Y] {
                    DebugSegment {
                        from: origin - axis * extent,
                        to: origin + axis * extent,
                        color: Color([0.2, 1.0, 0.65, 0.9]),
                    }
                    .draw(&mut state.frame, node_camera)?;
                }
            }
            if state.debug_draw.names {
                state.frame.push(DrawCommand::Text {
                    text: data.name.clone(),
                    position: origin + Vec2::new(7.0, -7.0),
                    scale: 1.0,
                    camera: node_camera,
                    color: Color([0.8, 1.0, 0.8, 0.95]),
                })?;
            }
            debug_nodes += 1;
        }
    }
    if state.development_tools && state.debug_draw.camera {
        let corners = [
            Vec2::ZERO,
            Vec2::new(size.x, 0.0),
            size,
            Vec2::new(0.0, size.y),
        ]
        .map(|point| camera.screen_to_world(point));
        for index in 0..4 {
            DebugSegment {
                from: corners[index],
                to: corners[(index + 1) % 4],
                color: Color([0.8, 0.4, 1.0, 0.9]),
            }
            .draw(&mut state.frame, camera)?;
        }
        let center = camera.screen_to_world(size * 0.5);
        let extent = 8.0 / camera.zoom.max(0.001);
        for axis in [Vec2::X, Vec2::Y] {
            DebugSegment {
                from: center - axis * extent,
                to: center + axis * extent,
                color: Color([1.0, 0.35, 0.8, 0.9]),
            }
            .draw(&mut state.frame, camera)?;
        }
    }
    Ok(())
}

/// Hit test only explicit interactive Control nodes, using the same native
/// transforms and camera selection as scene rendering. Last drawn wins.
pub(crate) fn hit_control(
    graph: &mut SceneGraph,
    width: u32,
    height: u32,
    point: Vec2,
) -> Result<Option<NodeId>> {
    ensure!(point.is_finite(), "pointer position must be finite");
    let camera = active_camera(graph, Vec2::new(width as f32, height as f32))?.unwrap_or_default();
    let mut hit = None;
    let mut pending = vec![(graph.root(), false)];
    while let Some((id, inherited_screen)) = pending.pop() {
        let node = graph.node(id)?;
        if !node.data().enabled || !node.data().visible {
            continue;
        }
        let data = node.data().clone();
        let screen = inherited_screen || data.kind == NodeKind::CanvasLayer;
        pending.extend(node.children().iter().rev().map(|child| (*child, screen)));
        if data.kind != NodeKind::Control || !boolean(&data, "interactive", false)? {
            continue;
        }
        let w = number(&data, "width", 0.0)?;
        let h = number(&data, "height", 0.0)?;
        if w <= 0.0 || h <= 0.0 {
            continue;
        }
        let matrix = graph.world_matrix(id)?;
        if matrix.determinant().abs() <= 0.000001 {
            continue;
        }
        let world = if screen {
            point
        } else {
            camera.screen_to_world(point)
        };
        let local = matrix.inverse().transform_point2(world);
        if (0.0..=w).contains(&local.x) && (0.0..=h).contains(&local.y) {
            hit = Some(id);
        }
    }
    Ok(hit)
}
