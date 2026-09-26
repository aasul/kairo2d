use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct InputState {
    keys: HashSet<String>,
    #[serde(default)]
    pressed_keys: HashSet<String>,
    #[serde(default)]
    pressed_buttons: HashSet<u8>,
    #[serde(default)]
    pressed_gamepad: std::collections::BTreeMap<u32, HashSet<String>>,
    pub gamepads: crate::gamepad::Gamepads,
    buttons: HashSet<u8>,
    pub mouse_x: f32,
    pub mouse_y: f32,
}

impl InputState {
    pub fn key_down(&self, key: &str) -> bool {
        self.keys.contains(&key.to_ascii_lowercase())
    }

    pub fn set_key(&mut self, key: &str, down: bool) -> bool {
        let key = key.to_ascii_lowercase();
        if down {
            let changed = self.keys.insert(key.clone());
            if changed {
                self.pressed_keys.insert(key);
            }
            changed
        } else {
            self.keys.remove(&key)
        }
    }

    pub fn mouse_down(&self, button: u8) -> bool {
        self.buttons.contains(&button)
    }

    pub fn set_button(&mut self, button: u8, down: bool) -> bool {
        if down {
            let changed = self.buttons.insert(button);
            if changed {
                self.pressed_buttons.insert(button);
            }
            changed
        } else {
            self.buttons.remove(&button)
        }
    }

    pub fn key_pressed(&self, key: &str) -> bool {
        self.pressed_keys.contains(key)
    }
    pub fn mouse_pressed(&self, button: u8) -> bool {
        self.pressed_buttons.contains(&button)
    }
    pub fn gamepad_pressed(&self, player: u32, button: &str) -> bool {
        self.pressed_gamepad
            .get(&player)
            .is_some_and(|buttons| buttons.contains(button))
    }
    pub fn mark_gamepad_pressed(&mut self, player: u32, button: &str) {
        self.pressed_gamepad
            .entry(player)
            .or_default()
            .insert(button.to_owned());
    }
    pub fn clear_edges(&mut self) {
        self.pressed_keys.clear();
        self.pressed_buttons.clear();
        self.pressed_gamepad.clear();
    }

    pub fn release_all(&mut self) -> (Vec<String>, Vec<u8>) {
        self.clear_edges();
        let mut keys: Vec<_> = self.keys.drain().collect();
        let mut buttons: Vec<_> = self.buttons.drain().collect();
        keys.sort();
        buttons.sort();
        (keys, buttons)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_key_events_are_not_new_presses() {
        let mut input = InputState::default();
        assert!(input.set_key("Space", true));
        assert!(!input.set_key("space", true));
        assert!(input.key_down("SPACE"));
        assert!(input.set_key("space", false));
        assert!(!input.key_down("space"));
    }

    #[test]
    fn losing_focus_releases_everything() {
        let mut input = InputState::default();
        input.set_key("d", true);
        input.set_button(1, true);
        let (keys, buttons) = input.release_all();
        assert_eq!(keys, ["d"]);
        assert_eq!(buttons, [1]);
        assert!(!input.key_down("d"));
        assert!(!input.mouse_down(1));
    }
}
