use gilrs::{Axis, Button, EventType, Gilrs};
use kairo_core::gamepad::{deadzone, Controller, Gamepads};

pub struct GamepadBackend {
    inner: Option<Gilrs>,
}

pub enum PadEvent {
    Connected(u32, String),
    Disconnected(u32),
    Button(u32, &'static str, bool),
}

impl Default for GamepadBackend {
    fn default() -> Self {
        let inner = match Gilrs::new() {
            Ok(inner) => Some(inner),
            Err(error) => {
                log::warn!("Gamepad input unavailable: {error}");
                None
            }
        };
        Self { inner }
    }
}

impl GamepadBackend {
    pub fn poll(&mut self, state: &mut Gamepads) -> Vec<PadEvent> {
        let Some(gilrs) = &mut self.inner else {
            return Vec::new();
        };
        let mut events = Vec::new();
        for _ in 0..1024 {
            let Some(event) = gilrs.next_event() else {
                break;
            };
            let Ok(index) = u32::try_from(usize::from(event.id)) else {
                continue;
            };
            let Some(id) = index.checked_add(1) else {
                continue;
            };
            match event.event {
                EventType::ButtonPressed(button, _) => {
                    if let Some(name) = button_name(button) {
                        events.push(PadEvent::Button(id, name, true));
                    }
                }
                EventType::ButtonReleased(button, _) => {
                    if let Some(name) = button_name(button) {
                        events.push(PadEvent::Button(id, name, false));
                    }
                }
                EventType::Connected => events.push(PadEvent::Connected(
                    id,
                    gilrs.gamepad(event.id).name().to_owned(),
                )),
                EventType::Disconnected => events.push(PadEvent::Disconnected(id)),
                _ => {}
            }
        }
        state.connected.clear();
        for (raw_id, pad) in gilrs.gamepads() {
            let Ok(index) = u32::try_from(usize::from(raw_id)) else {
                continue;
            };
            let Some(id) = index.checked_add(1) else {
                continue;
            };
            let mut controller = Controller {
                name: pad.name().to_owned(),
                left_stick: deadzone(
                    pad.value(Axis::LeftStickX),
                    -pad.value(Axis::LeftStickY),
                    0.15,
                ),
                right_stick: deadzone(
                    pad.value(Axis::RightStickX),
                    -pad.value(Axis::RightStickY),
                    0.15,
                ),
                left_trigger: pad
                    .button_data(Button::LeftTrigger2)
                    .map_or(0.0, |data| data.value()),
                right_trigger: pad
                    .button_data(Button::RightTrigger2)
                    .map_or(0.0, |data| data.value()),
                ..Default::default()
            };
            for button in [
                Button::South,
                Button::East,
                Button::West,
                Button::North,
                Button::Start,
                Button::Select,
                Button::Mode,
                Button::LeftTrigger,
                Button::RightTrigger,
                Button::LeftTrigger2,
                Button::RightTrigger2,
                Button::LeftThumb,
                Button::RightThumb,
                Button::DPadUp,
                Button::DPadDown,
                Button::DPadLeft,
                Button::DPadRight,
            ] {
                if pad.is_pressed(button) {
                    if let Some(name) = button_name(button) {
                        controller.buttons.insert(name.to_owned());
                    }
                }
            }
            state.connected.insert(id, controller);
        }
        events
    }
}

fn button_name(button: Button) -> Option<&'static str> {
    Some(match button {
        Button::South => "a",
        Button::East => "b",
        Button::West => "x",
        Button::North => "y",
        Button::Start => "start",
        Button::Select => "back",
        Button::Mode => "guide",
        Button::LeftTrigger => "left_shoulder",
        Button::RightTrigger => "right_shoulder",
        Button::LeftTrigger2 => "left_trigger",
        Button::RightTrigger2 => "right_trigger",
        Button::LeftThumb => "left_stick",
        Button::RightThumb => "right_stick",
        Button::DPadUp => "up",
        Button::DPadDown => "down",
        Button::DPadLeft => "left",
        Button::DPadRight => "right",
        _ => return None,
    })
}
