use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AnimationFrame {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub duration: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AnimationFile {
    pub texture: String,
    #[serde(default = "default_looping")]
    pub looping: bool,
    pub frames: Vec<AnimationFrame>,
}

fn default_looping() -> bool {
    true
}

/// The animation clock is independent of rendering and is stepped explicitly.
#[derive(Clone, Debug)]
pub struct Animation {
    frames: Vec<AnimationFrame>,
    ends: Vec<f64>,
    duration: f64,
    time: f64,
    speed: f64,
    looping: bool,
    paused: bool,
}

impl Animation {
    pub fn new(frames: Vec<AnimationFrame>, width: u32, height: u32) -> Result<Self> {
        ensure!(
            !frames.is_empty() && frames.len() <= 4096,
            "animation requires 1..=4096 frames"
        );
        let mut ends = Vec::with_capacity(frames.len());
        let mut duration = 0.0;
        for frame in &frames {
            ensure!(
                frame.w > 0 && frame.h > 0,
                "animation frames cannot be empty"
            );
            ensure!(
                u64::from(frame.x) + u64::from(frame.w) <= u64::from(width)
                    && u64::from(frame.y) + u64::from(frame.h) <= u64::from(height),
                "animation frame exceeds texture bounds"
            );
            ensure!(
                frame.duration.is_finite() && (0.000_001..=3600.0).contains(&frame.duration),
                "frame duration must be between one microsecond and one hour"
            );
            duration += frame.duration;
            ends.push(duration);
        }
        Ok(Self {
            frames,
            ends,
            duration,
            time: 0.0,
            speed: 1.0,
            looping: true,
            paused: false,
        })
    }

    pub fn update(&mut self, dt: f64) -> Result<()> {
        ensure!(
            dt.is_finite() && dt >= 0.0,
            "animation delta must be finite and nonnegative"
        );
        if self.paused {
            return Ok(());
        }
        let next = self.time + dt * self.speed;
        ensure!(next.is_finite(), "animation clock overflow");
        self.time = if self.looping {
            next.rem_euclid(self.duration)
        } else {
            next.min(self.duration)
        };
        Ok(())
    }

    pub fn frame_index(&self) -> usize {
        self.ends
            .partition_point(|end| *end <= self.time)
            .min(self.frames.len() - 1)
    }
    pub fn frame(&self) -> &AnimationFrame {
        &self.frames[self.frame_index()]
    }
    pub fn frames(&self) -> &[AnimationFrame] {
        &self.frames
    }
    pub fn finished(&self) -> bool {
        !self.looping && self.time >= self.duration
    }
    pub fn looping(&self) -> bool {
        self.looping
    }
    pub fn paused(&self) -> bool {
        self.paused
    }
    pub fn time(&self) -> f64 {
        self.time
    }
    pub fn set_looping(&mut self, looping: bool) {
        self.looping = looping;
    }
    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }
    pub fn restart(&mut self) {
        self.time = 0.0;
        self.paused = false;
    }
    pub fn set_speed(&mut self, speed: f64) -> Result<()> {
        ensure!(
            speed.is_finite() && (0.0..=100.0).contains(&speed),
            "animation speed must be in 0..=100"
        );
        self.speed = speed;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn clip() -> Animation {
        Animation::new(
            vec![
                AnimationFrame {
                    x: 0,
                    y: 0,
                    w: 8,
                    h: 8,
                    duration: 0.25,
                },
                AnimationFrame {
                    x: 8,
                    y: 0,
                    w: 8,
                    h: 8,
                    duration: 0.25,
                },
            ],
            16,
            8,
        )
        .unwrap()
    }
    #[test]
    fn looping_skips_arbitrarily_many_cycles_without_iteration() {
        let mut a = clip();
        a.update(1_000_000.25).unwrap();
        assert_eq!(a.frame_index(), 1);
        a.update(0.25).unwrap();
        assert_eq!(a.frame_index(), 0);
    }
    #[test]
    fn one_shots_hold_last_frame_and_pause_stops_time() {
        let mut a = clip();
        a.set_looping(false);
        a.set_paused(true);
        a.update(1.0).unwrap();
        assert_eq!(a.time(), 0.0);
        a.set_paused(false);
        a.update(1.0).unwrap();
        assert!(a.finished());
        assert_eq!(a.frame_index(), 1);
        a.restart();
        assert!(!a.finished());
    }
    #[test]
    fn invalid_clips_and_clocks_are_rejected() {
        assert!(Animation::new(vec![], 1, 1).is_err());
        let mut a = clip();
        assert!(a.update(f64::NAN).is_err());
        assert!(a.set_speed(-1.0).is_err());
        let mut frames = a.frames().to_vec();
        frames[0].x = u32::MAX;
        assert!(Animation::new(frames, 16, 8).is_err());
    }
}
