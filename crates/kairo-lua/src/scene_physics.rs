//! Binds authored body/collider nodes to the existing validated Rapier world.
//! Handles belong to this scene and are destroyed when the scene deactivates.
use anyhow::{bail, ensure, Context, Result};
use glam::{Mat3, Vec2};
use kairo_core::physics::BodyKind;
use kairo_core::scene_graph::{NodeFile, NodeId, NodeKind, SceneGraph};
use kairo_core::{BodyHandle, PhysicsWorld};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Shape {
    Rectangle(f32, f32),
    Circle(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct BodySpec {
    kind: BodyKind,
    shape: Shape,
    gravity_scale: f32,
}

struct BodyEntry {
    handle: BodyHandle,
    spec: BodySpec,
    position: Vec2,
    rotation: f32,
    velocity: Vec2,
}

#[derive(Default)]
pub(crate) struct ScenePhysics {
    bodies: HashMap<NodeId, BodyEntry>,
}

fn number(node: &NodeFile, key: &str, default: f32) -> Result<f32> {
    match node.properties.get(key) {
        None => Ok(default),
        Some(Value::Number(value)) => {
            let value = value.as_f64().context("invalid number")? as f32;
            ensure!(value.is_finite(), "{key} must be finite");
            Ok(value)
        }
        _ => bail!("{key} must be a number"),
    }
}

fn velocity(node: &NodeFile) -> Result<Vec2> {
    let Some(value) = node.properties.get("velocity") else {
        return Ok(Vec2::ZERO);
    };
    let read = |value: &Value| -> Result<f32> {
        let component = value
            .as_f64()
            .context("velocity component must be a number")? as f32;
        ensure!(component.is_finite(), "velocity component must be finite");
        Ok(component)
    };
    let (x, y) = match value {
        Value::Array(values) if values.len() == 2 => (read(&values[0])?, read(&values[1])?),
        Value::Object(values) => (
            read(values.get("x").context("velocity.x is required")?)?,
            read(values.get("y").context("velocity.y is required")?)?,
        ),
        _ => bail!("velocity must be [x, y] or {{x, y}}"),
    };
    Ok(Vec2::new(x, y))
}

fn spec(graph: &SceneGraph, id: NodeId, data: &NodeFile, world: Mat3) -> Result<BodySpec> {
    let kind = match data.kind {
        NodeKind::StaticBody2D => BodyKind::Static,
        NodeKind::DynamicBody2D | NodeKind::CharacterBody2D => BodyKind::Dynamic,
        NodeKind::PhysicsBody2D => match data.properties.get("body_type") {
            None | Some(Value::String(_))
                if data
                    .properties
                    .get("body_type")
                    .and_then(Value::as_str)
                    .unwrap_or("dynamic")
                    == "dynamic" =>
            {
                BodyKind::Dynamic
            }
            Some(Value::String(value)) if value == "static" => BodyKind::Static,
            _ => bail!("body_type must be 'dynamic' or 'static'"),
        },
        _ => bail!("not a physics body"),
    };
    let gravity_scale = if data.kind == NodeKind::CharacterBody2D {
        number(data, "gravity_scale", 0.0)?
    } else {
        number(data, "gravity_scale", 1.0)?
    };
    ensure!(gravity_scale >= 0.0, "gravity_scale cannot be negative");
    let children = graph.node(id)?.children();
    let mut colliders = children
        .iter()
        .filter_map(|child| graph.node(*child).ok())
        .filter(|child| child.data().kind == NodeKind::Collider2D && child.data().enabled);
    let collider = colliders
        .next()
        .context("body requires a direct Collider2D child")?;
    ensure!(
        colliders.next().is_none(),
        "body supports one active Collider2D child"
    );
    let transform = collider.data().transform;
    ensure!(
        transform.position == [0.0; 2]
            && transform.rotation == 0.0
            && transform.scale == [1.0; 2]
            && transform.pivot == [0.0; 2],
        "Collider2D offset/rotation/scale is not supported by native physics"
    );
    let x_axis = world.x_axis.truncate();
    let y_axis = world.y_axis.truncate();
    let sx = x_axis.length();
    let sy = y_axis.length();
    ensure!(sx > 0.0 && sy > 0.0, "physics body scale must be nonzero");
    ensure!(
        x_axis.dot(y_axis).abs() <= sx * sy * 0.001,
        "physics body cannot inherit shear"
    );
    let properties = &collider.data().properties;
    let shape = match properties
        .get("shape")
        .and_then(Value::as_str)
        .unwrap_or("rectangle")
    {
        "rectangle" => {
            let width = number(collider.data(), "width", 0.0)? * sx;
            let height = number(collider.data(), "height", 0.0)? * sy;
            ensure!(
                width > 0.0 && height > 0.0,
                "rectangle collider needs positive width and height"
            );
            Shape::Rectangle(width, height)
        }
        "circle" => {
            ensure!(
                (sx - sy).abs() <= sx * 0.001,
                "circle collider needs uniform world scale"
            );
            let radius = number(collider.data(), "radius", 0.0)? * sx;
            ensure!(radius > 0.0, "circle collider needs a positive radius");
            Shape::Circle(radius)
        }
        _ => bail!("collider shape must be 'rectangle' or 'circle'"),
    };
    Ok(BodySpec {
        kind,
        shape,
        gravity_scale,
    })
}

impl ScenePhysics {
    pub(crate) fn before(
        &mut self,
        graph: &mut SceneGraph,
        physics: &mut PhysicsWorld,
    ) -> Result<()> {
        let mut pending = vec![graph.root()];
        let mut desired = Vec::new();
        while let Some(id) = pending.pop() {
            let node = graph.node(id)?;
            if !node.data().enabled {
                continue;
            }
            let data = node.data().clone();
            pending.extend(node.children().iter().rev());
            if !matches!(
                data.kind,
                NodeKind::StaticBody2D
                    | NodeKind::DynamicBody2D
                    | NodeKind::CharacterBody2D
                    | NodeKind::PhysicsBody2D
            ) {
                continue;
            }
            let world = graph.world_matrix(id)?;
            let entry = (|| {
                let spec = spec(graph, id, &data, world)?;
                let position = graph.world_position(id)?;
                let axis = world.x_axis.truncate();
                let rotation = axis.y.atan2(axis.x);
                Ok::<_, anyhow::Error>((id, spec, position, rotation, velocity(&data)?))
            })()
            .with_context(|| {
                format!(
                    "scene '{}', node '{}': native physics",
                    graph.name(),
                    graph.path(id).unwrap_or_default()
                )
            })?;
            desired.push(entry);
        }
        let keep: HashSet<_> = desired.iter().map(|entry| entry.0).collect();
        for id in self.bodies.keys().copied().collect::<Vec<_>>() {
            if !keep.contains(&id) {
                let entry = self.bodies.remove(&id).expect("key exists");
                physics.destroy(entry.handle)?;
            }
        }
        for (id, spec, position, rotation, velocity) in desired {
            if self.bodies.get(&id).is_some_and(|entry| entry.spec != spec) {
                let entry = self.bodies.remove(&id).expect("entry exists");
                physics.destroy(entry.handle)?;
            }
            if let Some(entry) = self.bodies.get_mut(&id) {
                if (position - entry.position).length_squared() > 0.0001 {
                    physics.set_position(entry.handle, position.x, position.y)?;
                    entry.position = position;
                }
                if (rotation - entry.rotation).abs() > 0.0001 {
                    physics.set_rotation(entry.handle, rotation)?;
                    entry.rotation = rotation;
                }
                if (velocity - entry.velocity).length_squared() > 0.0001 {
                    physics.set_velocity(entry.handle, velocity.x, velocity.y)?;
                    entry.velocity = velocity;
                }
                continue;
            }
            let handle = match spec.shape {
                Shape::Rectangle(w, h) => {
                    physics.rectangle(spec.kind, position.x, position.y, w, h)?
                }
                Shape::Circle(radius) => {
                    physics.circle(spec.kind, position.x, position.y, radius)?
                }
            };
            physics.set_rotation(handle, rotation)?;
            physics.set_gravity_scale(handle, spec.gravity_scale)?;
            if spec.kind == BodyKind::Dynamic {
                physics.set_velocity(handle, velocity.x, velocity.y)?;
            }
            self.bodies.insert(
                id,
                BodyEntry {
                    handle,
                    spec,
                    position,
                    rotation,
                    velocity,
                },
            );
        }
        Ok(())
    }

    pub(crate) fn after(&mut self, graph: &mut SceneGraph, physics: &PhysicsWorld) -> Result<()> {
        // Parents must be written back before descendants calculate local poses.
        let mut pending = vec![graph.root()];
        while let Some(id) = pending.pop() {
            let children = graph.node(id)?.children().to_vec();
            if let Some(entry) = self.bodies.get_mut(&id) {
                if entry.spec.kind == BodyKind::Dynamic {
                    let (x, y) = physics.position(entry.handle)?;
                    let angle = physics.rotation(entry.handle)?;
                    let (vx, vy) = physics.velocity(entry.handle)?;
                    let world_position = Vec2::new(x, y);
                    let parent = graph.node(id)?.parent();
                    let parent_world = parent
                        .map(|id| graph.world_matrix(id))
                        .transpose()?
                        .unwrap_or(Mat3::IDENTITY);
                    ensure!(
                        parent_world.determinant().abs() > 0.000001,
                        "physics parent transform cannot be singular"
                    );
                    let parent_angle = parent_world.x_axis.y.atan2(parent_world.x_axis.x);
                    let mut transform = graph.node(id)?.data().transform;
                    transform.position = parent_world
                        .inverse()
                        .transform_point2(world_position)
                        .to_array();
                    transform.rotation = angle - parent_angle;
                    graph.set_transform(id, transform)?;
                    graph.set_property(id, "velocity", json!({"x": vx, "y": vy}))?;
                    entry.position = world_position;
                    entry.rotation = angle;
                    entry.velocity = Vec2::new(vx, vy);
                }
            }
            pending.extend(children.into_iter().rev());
        }
        Ok(())
    }

    pub(crate) fn deactivate(&mut self, physics: &mut PhysicsWorld) -> Result<()> {
        for (_, entry) in self.bodies.drain() {
            physics.destroy(entry.handle)?;
        }
        Ok(())
    }

    pub(crate) fn contacts(&self, physics: &PhysicsWorld) -> Vec<(NodeId, NodeId)> {
        let reverse: HashMap<_, _> = self
            .bodies
            .iter()
            .map(|(id, entry)| (entry.handle, *id))
            .collect();
        physics
            .touching_pairs()
            .into_iter()
            .filter_map(|(a, b)| Some((*reverse.get(&a)?, *reverse.get(&b)?)))
            .collect()
    }
}
