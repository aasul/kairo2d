use anyhow::{ensure, Result};
use glam::Vec2;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EmitterConfig {
    pub rate: f32,
    pub lifetime: [f32; 2],
    pub speed: [f32; 2],
    pub direction: f32,
    pub spread: f32,
    pub gravity: f32,
    pub drag: f32,
    pub size: [f32; 2],
    pub color_start: [f32; 4],
    pub color_end: [f32; 4],
    pub rotation: [f32; 2],
    pub max_particles: usize,
    pub seed: u64,
}
impl Default for EmitterConfig {
    fn default() -> Self {
        Self {
            rate: 40.0,
            lifetime: [0.2, 0.6],
            speed: [80.0, 220.0],
            direction: -std::f32::consts::FRAC_PI_2,
            spread: std::f32::consts::PI,
            gravity: 200.0,
            drag: 0.0,
            size: [4.0, 0.0],
            color_start: [1.0, 0.7, 0.2, 1.0],
            color_end: [1.0, 0.1, 0.05, 0.0],
            rotation: [0.0, 0.0],
            max_particles: 1024,
            seed: 1,
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ParticlePreset {
    pub texture: Option<String>,
    pub emitter: EmitterConfig,
}
impl EmitterConfig {
    pub fn validate(&self) -> Result<()> {
        for values in [
            &self.lifetime[..],
            &self.speed,
            &self.size,
            &self.color_start,
            &self.color_end,
            &self.rotation,
        ] {
            crate::finite("particle configuration", values)?;
        }
        crate::finite(
            "particle configuration",
            &[
                self.rate,
                self.direction,
                self.spread,
                self.gravity,
                self.drag,
            ],
        )?;
        ensure!(
            (0.0..=10000.0).contains(&self.rate) && (1..=8192).contains(&self.max_particles),
            "invalid particle rate/capacity"
        );
        ensure!(
            self.lifetime[0] > 0.0
                && self.lifetime[0] <= self.lifetime[1]
                && self.lifetime[1] <= 60.0,
            "particle lifetime must be an ordered range in (0,60]"
        );
        ensure!(
            self.speed[0] >= 0.0 && self.speed[0] <= self.speed[1] && self.speed[1] <= 10000.0,
            "invalid particle speed range"
        );
        ensure!(
            (0.0..=std::f32::consts::TAU).contains(&self.spread) && self.direction.abs() <= 10000.0,
            "particle direction/spread are radians"
        );
        ensure!(
            self.gravity.abs() <= 100000.0 && (0.0..=100.0).contains(&self.drag),
            "invalid gravity/drag"
        );
        ensure!(
            self.size.iter().all(|v| (0.0..=2048.0).contains(v)),
            "particle size must be 0..2048"
        );
        ensure!(
            self.color_start
                .iter()
                .chain(&self.color_end)
                .all(|v| (0.0..=1.0).contains(v)),
            "particle colors must be 0..1"
        );
        ensure!(
            self.rotation[0] <= self.rotation[1]
                && self.rotation.iter().all(|v| v.abs() <= 10000.0),
            "invalid rotation range"
        );
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct Particle {
    pub position: Vec2,
    pub velocity: Vec2,
    pub age: f32,
    pub lifetime: f32,
    pub rotation: f32,
}
pub struct Emitter {
    pub config: EmitterConfig,
    pub position: Vec2,
    particles: Vec<Particle>,
    remainder: f32,
    random: u64,
}
impl Emitter {
    pub fn new(config: EmitterConfig) -> Result<Self> {
        config.validate()?;
        let random = config.seed.max(1);
        Ok(Self {
            particles: Vec::with_capacity(config.max_particles),
            config,
            position: Vec2::ZERO,
            remainder: 0.0,
            random,
        })
    }
    pub fn reconfigure(&mut self, config: EmitterConfig) -> Result<()> {
        config.validate()?;
        self.particles.truncate(config.max_particles);
        self.config = config;
        Ok(())
    }
    fn unit(&mut self) -> f32 {
        self.random ^= self.random << 13;
        self.random ^= self.random >> 7;
        self.random ^= self.random << 17;
        ((self.random >> 40) as u32) as f32 / 16_777_216.0
    }
    pub fn set_position(&mut self, x: f32, y: f32) -> Result<()> {
        ensure!(
            x.is_finite() && y.is_finite() && x.abs() <= 1.0e9 && y.abs() <= 1.0e9,
            "invalid emitter position"
        );
        self.position = Vec2::new(x, y);
        Ok(())
    }
    pub fn emit(&mut self, count: usize) -> usize {
        let count = count.min(
            self.config
                .max_particles
                .saturating_sub(self.particles.len()),
        );
        for _ in 0..count {
            let angle = self.config.direction + (self.unit() - 0.5) * self.config.spread;
            let speed =
                self.config.speed[0] + self.unit() * (self.config.speed[1] - self.config.speed[0]);
            let lifetime = self.config.lifetime[0]
                + self.unit() * (self.config.lifetime[1] - self.config.lifetime[0]);
            let rotation = self.config.rotation[0]
                + self.unit() * (self.config.rotation[1] - self.config.rotation[0]);
            self.particles.push(Particle {
                position: self.position,
                velocity: Vec2::new(angle.cos(), angle.sin()) * speed,
                age: 0.0,
                lifetime,
                rotation,
            });
        }
        count
    }
    pub fn update(&mut self, dt: f32) -> Result<()> {
        ensure!(
            dt.is_finite() && (0.0..=1.0).contains(&dt),
            "particle delta must be 0..1 second"
        );
        let drag = (-self.config.drag * dt).exp();
        for particle in &mut self.particles {
            particle.age += dt;
            particle.velocity.y += self.config.gravity * dt;
            particle.velocity *= drag;
            particle.position += particle.velocity * dt;
        }
        self.particles
            .retain(|p| p.age < p.lifetime && p.position.is_finite());
        self.remainder += self.config.rate * dt;
        let count = self.remainder.floor() as usize;
        self.remainder -= count as f32;
        self.emit(count);
        Ok(())
    }
    pub fn clear(&mut self) {
        self.particles.clear();
        self.remainder = 0.0;
    }
    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }
    pub fn appearance(&self, particle: &Particle) -> (f32, [f32; 4]) {
        let t = (particle.age / particle.lifetime).clamp(0.0, 1.0);
        let size = self.config.size[0] + (self.config.size[1] - self.config.size[0]) * t;
        let mut color = [0.0; 4];
        for (i, c) in color.iter_mut().enumerate() {
            *c = self.config.color_start[i]
                + (self.config.color_end[i] - self.config.color_start[i]) * t;
        }
        (size, color)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capacity_lifetime_and_repeatable_seed() {
        let config = EmitterConfig {
            rate: 0.0,
            lifetime: [0.2, 0.2],
            max_particles: 8,
            ..Default::default()
        };
        let mut a = Emitter::new(config.clone()).unwrap();
        let mut b = Emitter::new(config).unwrap();
        assert_eq!(a.emit(100), 8);
        b.emit(8);
        assert_eq!(a.particles()[0].velocity, b.particles()[0].velocity);
        a.update(0.25).unwrap();
        assert!(a.particles().is_empty());
        assert!(a.update(f32::NAN).is_err());
    }
    #[test]
    fn bad_ranges_are_rejected_without_reconfiguring() {
        let mut emitter = Emitter::new(EmitterConfig::default()).unwrap();
        assert!(emitter
            .reconfigure(EmitterConfig {
                lifetime: [2.0, 1.0],
                ..Default::default()
            })
            .is_err());
        assert_eq!(emitter.config.lifetime, [0.2, 0.6]);
    }
}
