use crate::{config::PhysicsConfig, finite, BodyHandle};
use anyhow::{bail, ensure, Context, Result};
use rapier2d::prelude::*;
use std::collections::HashMap;

const STEP: f32 = 1.0 / 120.0;
const MAX_STEPS: usize = 12;

#[derive(Clone, Copy, Debug)]
pub enum BodyKind {
    Dynamic,
    Static,
}

impl BodyKind {
    pub fn parse(name: &str) -> Result<Self> {
        match name {
            "dynamic" => Ok(Self::Dynamic),
            "static" => Ok(Self::Static),
            _ => bail!("body type must be 'dynamic' or 'static'"),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BodySnapshot {
    handle: BodyHandle,
    position: [f32; 2],
    rotation: f32,
    velocity: [f32; 2],
    angular_velocity: f32,
    sleeping: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PhysicsSnapshot {
    bodies: Vec<BodySnapshot>,
    gravity: [f32; 2],
    accumulator: f32,
    scale: f32,
}

pub struct PhysicsWorld {
    pipeline: PhysicsPipeline,
    gravity: Vector<Real>,
    integration: IntegrationParameters,
    islands: IslandManager,
    broad_phase: BroadPhaseMultiSap,
    narrow_phase: NarrowPhase,
    bodies: RigidBodySet,
    colliders: ColliderSet,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    ccd_solver: CCDSolver,
    handles: HashMap<BodyHandle, RigidBodyHandle>,
    scale: f32,
    accumulator: f32,
}

impl PhysicsWorld {
    pub fn new(config: &PhysicsConfig) -> Result<Self> {
        finite(
            "physics configuration",
            &[config.gravity_x, config.gravity_y, config.pixels_per_meter],
        )?;
        ensure!(
            config.pixels_per_meter > 0.0,
            "pixels_per_meter must be positive"
        );
        let gravity = Vector::new(
            config.gravity_x / config.pixels_per_meter,
            config.gravity_y / config.pixels_per_meter,
        );
        finite("scaled gravity", &[gravity.x, gravity.y])?;
        Ok(Self {
            pipeline: PhysicsPipeline::new(),
            gravity,
            integration: IntegrationParameters {
                dt: STEP,
                ..Default::default()
            },
            islands: IslandManager::new(),
            broad_phase: BroadPhaseMultiSap::new(),
            narrow_phase: NarrowPhase::new(),
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            handles: HashMap::new(),
            scale: config.pixels_per_meter,
            accumulator: 0.0,
        })
    }

    pub fn rectangle(
        &mut self,
        kind: BodyKind,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> Result<BodyHandle> {
        finite("rectangle", &[x, y, width, height])?;
        ensure!(
            width > 0.0 && height > 0.0,
            "body dimensions must be positive"
        );
        let half_width = width / self.scale / 2.0;
        let half_height = height / self.scale / 2.0;
        finite("scaled body dimensions", &[half_width, half_height])?;
        ensure!(
            half_width > 0.0 && half_height > 0.0,
            "body dimensions are too small for pixels_per_meter"
        );
        self.insert(kind, x, y, ColliderBuilder::cuboid(half_width, half_height))
    }

    pub fn circle(&mut self, kind: BodyKind, x: f32, y: f32, radius: f32) -> Result<BodyHandle> {
        finite("circle", &[x, y, radius])?;
        ensure!(radius > 0.0, "body radius must be positive");
        let radius = radius / self.scale;
        finite("scaled body radius", &[radius])?;
        ensure!(
            radius > 0.0,
            "body radius is too small for pixels_per_meter"
        );
        self.insert(kind, x, y, ColliderBuilder::ball(radius))
    }

    fn insert(
        &mut self,
        kind: BodyKind,
        x: f32,
        y: f32,
        collider: ColliderBuilder,
    ) -> Result<BodyHandle> {
        ensure!(
            self.handles.len() < 10_000,
            "physics body limit reached (10000)"
        );
        let position = self.scaled_vector("body position", x, y)?;
        let handle = BodyHandle::allocate()?;
        let builder = match kind {
            BodyKind::Dynamic => RigidBodyBuilder::dynamic(),
            BodyKind::Static => RigidBodyBuilder::fixed(),
        };
        let body = self
            .bodies
            .insert(builder.translation(position).ccd_enabled(true).build());
        self.colliders.insert_with_parent(
            collider.friction(0.7).restitution(0.2).build(),
            body,
            &mut self.bodies,
        );
        self.handles.insert(handle, body);
        Ok(handle)
    }

    fn scaled_vector(&self, name: &str, x: f32, y: f32) -> Result<Vector<Real>> {
        finite(name, &[x, y])?;
        let value = Vector::new(x / self.scale, y / self.scale);
        finite("scaled physics vector", &[value.x, value.y])?;
        Ok(value)
    }

    fn raw_handle(&self, handle: BodyHandle) -> Result<RigidBodyHandle> {
        self.handles
            .get(&handle)
            .copied()
            .context("physics body has been destroyed or belongs to another world")
    }

    fn body(&self, handle: BodyHandle) -> Result<&RigidBody> {
        self.bodies
            .get(self.raw_handle(handle)?)
            .context("physics body no longer exists")
    }

    fn body_mut(&mut self, handle: BodyHandle) -> Result<&mut RigidBody> {
        let handle = self.raw_handle(handle)?;
        self.bodies
            .get_mut(handle)
            .context("physics body no longer exists")
    }

    pub fn position(&self, handle: BodyHandle) -> Result<(f32, f32)> {
        let p = self.body(handle)?.translation();
        Ok((p.x * self.scale, p.y * self.scale))
    }

    pub fn set_position(&mut self, handle: BodyHandle, x: f32, y: f32) -> Result<()> {
        let position = self.scaled_vector("position", x, y)?;
        self.body_mut(handle)?.set_translation(position, true);
        Ok(())
    }

    pub fn rotation(&self, handle: BodyHandle) -> Result<f32> {
        Ok(self.body(handle)?.rotation().angle())
    }

    pub fn set_rotation(&mut self, handle: BodyHandle, angle: f32) -> Result<()> {
        finite("rotation", &[angle])?;
        self.body_mut(handle)?
            .set_rotation(Rotation::new(angle), true);
        Ok(())
    }

    pub fn velocity(&self, handle: BodyHandle) -> Result<(f32, f32)> {
        let v = self.body(handle)?.linvel();
        Ok((v.x * self.scale, v.y * self.scale))
    }

    pub fn set_velocity(&mut self, handle: BodyHandle, x: f32, y: f32) -> Result<()> {
        let velocity = self.scaled_vector("velocity", x, y)?;
        self.body_mut(handle)?.set_linvel(velocity, true);
        Ok(())
    }

    pub fn apply_impulse(&mut self, handle: BodyHandle, x: f32, y: f32) -> Result<()> {
        let impulse = self.scaled_vector("impulse", x, y)?;
        self.body_mut(handle)?.apply_impulse(impulse, true);
        Ok(())
    }

    /// Contact state from the most recent fixed step, not a predictive overlap query.
    pub fn is_touching(&self, first: BodyHandle, second: BodyHandle) -> Result<bool> {
        let first_body = self.body(first)?;
        let second_body = self.body(second)?;
        if first == second {
            return Ok(false);
        }
        for &a in first_body.colliders() {
            for &b in second_body.colliders() {
                if self
                    .narrow_phase
                    .contact_pair(a, b)
                    .is_some_and(|pair| pair.has_any_active_contact)
                {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    pub fn destroy(&mut self, handle: BodyHandle) -> Result<()> {
        let raw = self.raw_handle(handle)?;
        self.handles.remove(&handle);
        self.bodies.remove(
            raw,
            &mut self.islands,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            true,
        );
        Ok(())
    }

    pub fn set_gravity(&mut self, x: f32, y: f32) -> Result<()> {
        self.gravity = self.scaled_vector("gravity", x, y)?;
        for (_, body) in self.bodies.iter_mut() {
            body.wake_up(true);
        }
        Ok(())
    }

    pub fn step(&mut self, dt: f32) -> Result<()> {
        finite("delta time", &[dt])?;
        ensure!(dt >= 0.0, "delta time cannot be negative");
        self.accumulator += dt.min(0.1);
        let mut steps = 0;
        while self.accumulator >= STEP && steps < MAX_STEPS {
            self.pipeline.step(
                &self.gravity,
                &self.integration,
                &mut self.islands,
                &mut self.broad_phase,
                &mut self.narrow_phase,
                &mut self.bodies,
                &mut self.colliders,
                &mut self.impulse_joints,
                &mut self.multibody_joints,
                &mut self.ccd_solver,
                None,
                &(),
                &(),
            );
            self.accumulator -= STEP;
            steps += 1;
        }
        // Bound catch-up work after a debugger pause without changing the fixed step.
        self.accumulator = self.accumulator.min(STEP);
        Ok(())
    }

    pub fn snapshot(&self) -> Result<PhysicsSnapshot> {
        let mut bodies = Vec::with_capacity(self.handles.len());
        for &handle in self.handles.keys() {
            let body = self.body(handle)?;
            bodies.push(BodySnapshot {
                handle,
                position: [body.translation().x, body.translation().y],
                rotation: body.rotation().angle(),
                velocity: [body.linvel().x, body.linvel().y],
                angular_velocity: body.angvel(),
                sleeping: body.is_sleeping(),
            });
        }
        Ok(PhysicsSnapshot {
            bodies,
            gravity: [self.gravity.x, self.gravity.y],
            accumulator: self.accumulator,
            scale: self.scale,
        })
    }

    /// Restore body kinematics, not Rapier's internal solver/contact caches.
    /// Topology changes invalidate old snapshots; nothing changes on validation failure.
    pub fn restore(&mut self, snapshot: &PhysicsSnapshot) -> Result<()> {
        ensure!(snapshot.scale == self.scale && snapshot.bodies.len() == self.handles.len(),
            "replay cannot restore a changed physics topology; clear history after creating/destroying bodies");
        for body in &snapshot.bodies {
            self.raw_handle(body.handle)?;
            finite(
                "physics snapshot",
                &[
                    body.position[0],
                    body.position[1],
                    body.rotation,
                    body.velocity[0],
                    body.velocity[1],
                    body.angular_velocity,
                ],
            )?;
        }
        finite("snapshot gravity", &snapshot.gravity)?;
        ensure!(
            snapshot.accumulator.is_finite() && (0.0..=STEP).contains(&snapshot.accumulator),
            "invalid physics snapshot accumulator"
        );
        for saved in &snapshot.bodies {
            let body = self.body_mut(saved.handle)?;
            body.set_translation(Vector::new(saved.position[0], saved.position[1]), true);
            body.set_rotation(Rotation::new(saved.rotation), true);
            body.set_linvel(Vector::new(saved.velocity[0], saved.velocity[1]), true);
            body.set_angvel(saved.angular_velocity, true);
            if saved.sleeping {
                body.sleep();
            }
        }
        self.gravity = Vector::new(snapshot.gravity[0], snapshot.gravity[1]);
        self.accumulator = snapshot.accumulator;
        Ok(())
    }

    pub fn debug_segments(&self, velocities: bool) -> Vec<crate::debug_draw::DebugSegment> {
        use crate::{debug_draw::DebugSegment, Color};
        use glam::Vec2;
        let mut segments = Vec::new();
        for (_, body) in self.bodies.iter().take(256) {
            let color = if body.is_sleeping() {
                Color([0.55, 0.6, 0.65, 0.9])
            } else if body.is_fixed() {
                Color([0.35, 0.65, 1.0, 0.9])
            } else {
                Color([0.3, 1.0, 0.5, 0.9])
            };
            let center = Vec2::new(body.translation().x, body.translation().y) * self.scale;
            for delta in [Vec2::new(3.0, 0.0), Vec2::new(0.0, 3.0)] {
                segments.push(DebugSegment {
                    from: center - delta,
                    to: center + delta,
                    color,
                });
            }
            if velocities {
                segments.push(DebugSegment {
                    from: center,
                    to: center + Vec2::new(body.linvel().x, body.linvel().y) * self.scale * 0.1,
                    color: Color([1.0, 0.4, 0.35, 0.9]),
                });
            }
            for handle in body.colliders() {
                let Some(collider) = self.colliders.get(*handle) else {
                    continue;
                };
                let local: Vec<_> = if let Some(shape) = collider.shape().as_cuboid() {
                    let h = shape.half_extents;
                    vec![[-h.x, -h.y], [h.x, -h.y], [h.x, h.y], [-h.x, h.y]]
                } else if let Some(shape) = collider.shape().as_ball() {
                    (0..24)
                        .map(|i| {
                            let angle = i as f32 * std::f32::consts::TAU / 24.0;
                            [angle.cos() * shape.radius, angle.sin() * shape.radius]
                        })
                        .collect()
                } else {
                    continue;
                };
                let points: Vec<_> = local
                    .into_iter()
                    .map(|p| {
                        let point = collider.position().transform_point(&Point::new(p[0], p[1]));
                        Vec2::new(point.x, point.y) * self.scale
                    })
                    .collect();
                for i in 0..points.len() {
                    segments.push(DebugSegment {
                        from: points[i],
                        to: points[(i + 1) % points.len()],
                        color,
                    });
                }
            }
        }
        segments
    }

    pub fn body_count(&self) -> usize {
        self.handles.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshots_restore_kinematics_but_reject_topology_changes() {
        let mut world = PhysicsWorld::new(&PhysicsConfig::default()).unwrap();
        let body = world.circle(BodyKind::Dynamic, 20.0, 30.0, 8.0).unwrap();
        let saved = world.snapshot().unwrap();
        world.set_position(body, 100.0, 200.0).unwrap();
        world.restore(&saved).unwrap();
        assert!((world.position(body).unwrap().0 - 20.0).abs() < 0.001);
        world.circle(BodyKind::Dynamic, 0.0, 0.0, 8.0).unwrap();
        assert!(world.restore(&saved).is_err());
    }

    #[test]
    fn gravity_moves_dynamic_bodies_in_pixel_units() {
        let mut world = PhysicsWorld::new(&PhysicsConfig::default()).unwrap();
        let body = world
            .rectangle(BodyKind::Dynamic, 100.0, 0.0, 32.0, 32.0)
            .unwrap();
        for _ in 0..120 {
            world.step(STEP).unwrap();
        }
        let (x, y) = world.position(body).unwrap();
        assert!((x - 100.0).abs() < 0.01);
        assert!((y - 490.0).abs() < 10.0, "y = {y}");
    }

    #[test]
    fn floor_stops_a_falling_box() {
        let mut world = PhysicsWorld::new(&PhysicsConfig::default()).unwrap();
        world
            .rectangle(BodyKind::Static, 100.0, 300.0, 500.0, 20.0)
            .unwrap();
        let body = world
            .rectangle(BodyKind::Dynamic, 100.0, 20.0, 20.0, 20.0)
            .unwrap();
        for _ in 0..600 {
            world.step(STEP).unwrap();
        }
        assert!((world.position(body).unwrap().1 - 280.0).abs() < 3.0);
    }

    #[test]
    fn unit_conversion_rejects_overflow_before_entering_rapier() {
        let config = PhysicsConfig {
            pixels_per_meter: f32::MIN_POSITIVE,
            ..Default::default()
        };
        assert!(PhysicsWorld::new(&config).is_err());
        let config = PhysicsConfig {
            gravity_y: 0.0,
            ..config
        };
        let mut world = PhysicsWorld::new(&config).unwrap();
        assert!(world.circle(BodyKind::Dynamic, 0.0, 0.0, 100.0).is_err());
        assert!(world.set_gravity(f32::MAX, 0.0).is_err());
        assert_eq!(world.body_count(), 0);
    }

    #[test]
    fn destroyed_and_foreign_handles_are_rejected() {
        let mut world = PhysicsWorld::new(&PhysicsConfig::default()).unwrap();
        let mut other = PhysicsWorld::new(&PhysicsConfig::default()).unwrap();
        let body = world.circle(BodyKind::Dynamic, 0.0, 0.0, 10.0).unwrap();
        other.circle(BodyKind::Dynamic, 0.0, 0.0, 10.0).unwrap();
        assert!(other.position(body).is_err());
        world.destroy(body).unwrap();
        assert!(world.position(body).is_err());
        assert!(world.destroy(body).is_err());
    }

    #[test]
    fn contacts_are_reported_after_stepping_and_destroyed_bodies_error() {
        let mut world = PhysicsWorld::new(&PhysicsConfig::default()).unwrap();
        let floor = world
            .rectangle(BodyKind::Static, 100.0, 300.0, 500.0, 20.0)
            .unwrap();
        let body = world.circle(BodyKind::Dynamic, 100.0, 10.0, 10.0).unwrap();
        assert!(!world.is_touching(body, floor).unwrap());
        for _ in 0..600 {
            world.step(STEP).unwrap();
        }
        assert!(world.is_touching(body, floor).unwrap());
        world.destroy(body).unwrap();
        assert!(world.is_touching(body, floor).is_err());
    }
}
