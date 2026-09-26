use winit::event::MouseButton;
use winit::keyboard::KeyCode;

pub fn key_name(code: KeyCode) -> Option<&'static str> {
    use KeyCode::*;
    Some(match code {
        KeyA => "a",
        KeyB => "b",
        KeyC => "c",
        KeyD => "d",
        KeyE => "e",
        KeyF => "f",
        KeyG => "g",
        KeyH => "h",
        KeyI => "i",
        KeyJ => "j",
        KeyK => "k",
        KeyL => "l",
        KeyM => "m",
        KeyN => "n",
        KeyO => "o",
        KeyP => "p",
        KeyQ => "q",
        KeyR => "r",
        KeyS => "s",
        KeyT => "t",
        KeyU => "u",
        KeyV => "v",
        KeyW => "w",
        KeyX => "x",
        KeyY => "y",
        KeyZ => "z",
        Digit0 => "0",
        Digit1 => "1",
        Digit2 => "2",
        Digit3 => "3",
        Digit4 => "4",
        Digit5 => "5",
        Digit6 => "6",
        Digit7 => "7",
        Digit8 => "8",
        Digit9 => "9",
        ArrowLeft => "left",
        ArrowRight => "right",
        ArrowUp => "up",
        ArrowDown => "down",
        Space => "space",
        Enter => "enter",
        Escape => "escape",
        Tab => "tab",
        Backspace => "backspace",
        Delete => "delete",
        Insert => "insert",
        Home => "home",
        End => "end",
        PageUp => "pageup",
        PageDown => "pagedown",
        ShiftLeft => "lshift",
        ShiftRight => "rshift",
        ControlLeft => "lctrl",
        ControlRight => "rctrl",
        AltLeft => "lalt",
        AltRight => "ralt",
        SuperLeft => "lsuper",
        SuperRight => "rsuper",
        CapsLock => "capslock",
        NumLock => "numlock",
        ScrollLock => "scrolllock",
        F1 => "f1",
        F2 => "f2",
        F3 => "f3",
        F4 => "f4",
        F5 => "f5",
        F6 => "f6",
        F7 => "f7",
        F8 => "f8",
        F9 => "f9",
        F10 => "f10",
        F11 => "f11",
        F12 => "f12",
        Minus => "-",
        Equal => "=",
        BracketLeft => "[",
        BracketRight => "]",
        Backslash => "\\",
        Semicolon => ";",
        Quote => "'",
        Comma => ",",
        Period => ".",
        Slash => "/",
        Backquote => "`",
        Numpad0 => "kp0",
        Numpad1 => "kp1",
        Numpad2 => "kp2",
        Numpad3 => "kp3",
        Numpad4 => "kp4",
        Numpad5 => "kp5",
        Numpad6 => "kp6",
        Numpad7 => "kp7",
        Numpad8 => "kp8",
        Numpad9 => "kp9",
        NumpadAdd => "kp+",
        NumpadSubtract => "kp-",
        NumpadMultiply => "kp*",
        NumpadDivide => "kp/",
        NumpadDecimal => "kp.",
        NumpadEnter => "kpenter",
        _ => return None,
    })
}

pub fn button_number(button: MouseButton) -> Option<u8> {
    match button {
        MouseButton::Left => Some(1),
        MouseButton::Right => Some(2),
        MouseButton::Middle => Some(3),
        MouseButton::Back => Some(4),
        MouseButton::Forward => Some(5),
        MouseButton::Other(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_names_match_the_lua_conventions() {
        assert_eq!(key_name(KeyCode::KeyD), Some("d"));
        assert_eq!(key_name(KeyCode::Space), Some("space"));
        assert_eq!(key_name(KeyCode::ShiftLeft), Some("lshift"));
        assert_eq!(button_number(MouseButton::Right), Some(2));
        assert_eq!(button_number(MouseButton::Other(99)), None);
    }
}
