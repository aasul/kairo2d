use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Controller {
    pub name: String,
    pub buttons: BTreeSet<String>,
    pub left_stick: [f32; 2],
    pub right_stick: [f32; 2],
    pub left_trigger: f32,
    pub right_trigger: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Gamepads {
    pub connected: BTreeMap<u32, Controller>,
}

pub fn deadzone(x: f32, y: f32, zone: f32) -> [f32; 2] {
    let magnitude = x.hypot(y);
    if !magnitude.is_finite() || magnitude <= zone || !(0.0..1.0).contains(&zone) {
        return [0.0; 2];
    }
    let scaled = ((magnitude.min(1.0) - zone) / (1.0 - zone)).clamp(0.0, 1.0);
    [x / magnitude * scaled, y / magnitude * scaled]
}

pub const BUTTONS: &[&str] = &[
    "a",
    "b",
    "x",
    "y",
    "start",
    "back",
    "guide",
    "left_shoulder",
    "right_shoulder",
    "left_trigger",
    "right_trigger",
    "left_stick",
    "right_stick",
    "up",
    "down",
    "left",
    "right",
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn radial_deadzone_is_continuous_and_preserves_direction() {
        assert_eq!(deadzone(0.05, 0.0, 0.15), [0.0, 0.0]);
        assert_eq!(deadzone(1.0, 0.0, 0.15), [1.0, 0.0]);
        let d = deadzone(1.0, 1.0, 0.15);
        assert!((d[0].hypot(d[1]) - 1.0).abs() < 0.0001);
        assert_eq!(deadzone(f32::NAN, 0.0, 0.15), [0.0, 0.0]);
    }
}
