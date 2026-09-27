use crate::InputState;
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ActionBinding {
    pub keyboard: Vec<String>,
    pub mouse: Vec<u8>,
    pub gamepad: Vec<String>,
    pub player: u32,
}
impl Default for ActionBinding {
    fn default() -> Self {
        Self {
            keyboard: Vec::new(),
            mouse: Vec::new(),
            gamepad: Vec::new(),
            player: 1,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AxisBinding {
    pub negative: Vec<String>,
    pub positive: Vec<String>,
    pub gamepad_axis: Option<String>,
    pub player: u32,
    pub dead_zone: f32,
}
impl Default for AxisBinding {
    fn default() -> Self {
        Self {
            negative: Vec::new(),
            positive: Vec::new(),
            gamepad_axis: None,
            player: 1,
            dead_zone: 0.15,
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct InputBindings {
    pub actions: BTreeMap<String, ActionBinding>,
    pub axes: BTreeMap<String, AxisBinding>,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct ActionState {
    pub held: bool,
    pub pressed: bool,
    pub released: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Actions {
    pub bindings: InputBindings,
    states: BTreeMap<String, ActionState>,
    axes: BTreeMap<String, f32>,
}
fn name(value: &str) -> Result<()> {
    ensure!(
        !value.is_empty()
            && value.len() <= 64
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.')),
        "input names use 1..64 letters, digits, '_', '-', '.'"
    );
    Ok(())
}
fn keys(values: &[String]) -> Result<()> {
    ensure!(values.len() <= 16, "at most 16 keys per binding");
    for key in values {
        ensure!(
            !key.is_empty()
                && key.len() <= 32
                && key.bytes().all(|b| b.is_ascii_lowercase()
                    || b.is_ascii_digit()
                    || matches!(b, b'_' | b'-')),
            "key names must be lowercase ASCII"
        );
    }
    Ok(())
}
impl InputBindings {
    pub fn parse(source: &str) -> Result<Self> {
        ensure!(source.len() <= 256 * 1024, "input.toml exceeds 256 KiB");
        let value: Self = toml::from_str(source).context("invalid input.toml")?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.actions.len() <= 128 && self.axes.len() <= 64,
            "input mapping limit exceeded"
        );
        for (id, action) in &self.actions {
            name(id)?;
            keys(&action.keyboard)?;
            ensure!(
                action.mouse.len() <= 5 && action.mouse.iter().all(|v| (1..=5).contains(v)),
                "mouse bindings must be 1..5"
            );
            ensure!(
                action.gamepad.len() <= 16
                    && action
                        .gamepad
                        .iter()
                        .all(|b| crate::gamepad::BUTTONS.contains(&b.as_str())),
                "unknown gamepad button"
            );
            ensure!(action.player > 0, "gamepad IDs are one-based");
        }
        for (id, axis) in &self.axes {
            name(id)?;
            keys(&axis.negative)?;
            keys(&axis.positive)?;
            ensure!(
                axis.player > 0
                    && axis.dead_zone.is_finite()
                    && (0.0..1.0).contains(&axis.dead_zone),
                "invalid axis player/dead zone"
            );
            ensure!(
                axis.gamepad_axis.as_deref().is_none_or(|v| [
                    "left_x",
                    "left_y",
                    "right_x",
                    "right_y",
                    "left_trigger",
                    "right_trigger"
                ]
                .contains(&v)),
                "unknown gamepad axis"
            );
        }
        Ok(())
    }
}
impl Actions {
    pub fn replace(&mut self, bindings: InputBindings) -> Result<()> {
        bindings.validate()?;
        self.bindings = bindings;
        self.states.clear();
        self.axes.clear();
        Ok(())
    }
    pub fn sample(&mut self, input: &InputState) {
        for (name, binding) in &self.bindings.actions {
            let pad = input.gamepads.connected.get(&binding.player);
            let held = binding.keyboard.iter().any(|k| input.key_down(k))
                || binding.mouse.iter().any(|b| input.mouse_down(*b))
                || binding
                    .gamepad
                    .iter()
                    .any(|b| pad.is_some_and(|p| p.buttons.contains(b)));
            let tap = binding.keyboard.iter().any(|k| input.key_pressed(k))
                || binding.mouse.iter().any(|b| input.mouse_pressed(*b))
                || binding
                    .gamepad
                    .iter()
                    .any(|b| input.gamepad_pressed(binding.player, b));
            let state = self.states.entry(name.clone()).or_default();
            state.pressed = !state.held && (held || tap);
            state.released = !held && (state.held || tap);
            state.held = held;
        }
        for (name, binding) in &self.bindings.axes {
            let negative = binding.negative.iter().any(|k| input.key_down(k));
            let positive = binding.positive.iter().any(|k| input.key_down(k));
            let digital = i32::from(positive) as f32 - i32::from(negative) as f32;
            let analog = input
                .gamepads
                .connected
                .get(&binding.player)
                .map_or(0.0, |pad| match binding.gamepad_axis.as_deref() {
                    Some("left_x") => pad.left_stick[0],
                    Some("left_y") => pad.left_stick[1],
                    Some("right_x") => pad.right_stick[0],
                    Some("right_y") => pad.right_stick[1],
                    Some("left_trigger") => pad.left_trigger,
                    Some("right_trigger") => pad.right_trigger,
                    _ => 0.0,
                });
            let analog = scalar_deadzone(analog, binding.dead_zone);
            self.axes
                .insert(name.clone(), if digital != 0.0 { digital } else { analog });
        }
    }
    pub fn action(&self, name: &str) -> Result<ActionState> {
        ensure!(
            self.bindings.actions.contains_key(name),
            "unknown input action '{name}'"
        );
        Ok(self.states.get(name).copied().unwrap_or_default())
    }
    pub fn axis(&self, name: &str) -> Result<f32> {
        ensure!(
            self.bindings.axes.contains_key(name),
            "unknown input axis '{name}'"
        );
        Ok(self.axes.get(name).copied().unwrap_or_default())
    }
}
pub fn scalar_deadzone(value: f32, zone: f32) -> f32 {
    if !value.is_finite() || !zone.is_finite() || !(0.0..1.0).contains(&zone) || value.abs() <= zone
    {
        return 0.0;
    }
    value.signum() * ((value.abs().min(1.0) - zone) / (1.0 - zone))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn input_roundtrip_and_invalid_bindings() {
        let bindings = InputBindings::parse(
            "[actions.jump]\nkeyboard=['space']\n[axes.move_x]\nnegative=['a']\npositive=['d']",
        )
        .unwrap();
        InputBindings::parse(&toml::to_string(&bindings).unwrap()).unwrap();
        assert!(InputBindings::parse("[axes.bad]\ndead_zone=1").is_err());
        assert!(InputBindings::parse("[actions.bad]\ngamepad=['wat']").is_err());
    }
    #[test]
    fn short_taps_survive_between_frames_and_rebinding_clears_edges() {
        let mut map = Actions::default();
        map.replace(InputBindings::parse("[actions.jump]\nkeyboard=['space']").unwrap())
            .unwrap();
        let mut raw = InputState::default();
        raw.set_key("space", true);
        raw.set_key("space", false);
        map.sample(&raw);
        let state = map.action("jump").unwrap();
        assert!(state.pressed && state.released && !state.held);
        raw.clear_edges();
        map.sample(&raw);
        assert!(!map.action("jump").unwrap().pressed);
        assert!(map.action("missing").is_err());
        map.replace(InputBindings::default()).unwrap();
        assert!(map.action("jump").is_err());
    }
    #[test]
    fn axes_cancel_and_deadzone_is_continuous() {
        assert_eq!(scalar_deadzone(0.1, 0.2), 0.0);
        assert_eq!(scalar_deadzone(-1.0, 0.2), -1.0);
        let mut map = Actions::default();
        map.replace(InputBindings::parse("[axes.x]\nnegative=['a']\npositive=['d']").unwrap())
            .unwrap();
        let mut raw = InputState::default();
        raw.set_key("a", true);
        raw.set_key("d", true);
        map.sample(&raw);
        assert_eq!(map.axis("x").unwrap(), 0.0);
    }
}
