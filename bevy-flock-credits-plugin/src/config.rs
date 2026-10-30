use bevy::prelude::{KeyCode, Resource};
use serde::Deserialize;

const LETTER_KEYS: [KeyCode; 26] = [
    KeyCode::KeyA,
    KeyCode::KeyB,
    KeyCode::KeyC,
    KeyCode::KeyD,
    KeyCode::KeyE,
    KeyCode::KeyF,
    KeyCode::KeyG,
    KeyCode::KeyH,
    KeyCode::KeyI,
    KeyCode::KeyJ,
    KeyCode::KeyK,
    KeyCode::KeyL,
    KeyCode::KeyM,
    KeyCode::KeyN,
    KeyCode::KeyO,
    KeyCode::KeyP,
    KeyCode::KeyQ,
    KeyCode::KeyR,
    KeyCode::KeyS,
    KeyCode::KeyT,
    KeyCode::KeyU,
    KeyCode::KeyV,
    KeyCode::KeyW,
    KeyCode::KeyX,
    KeyCode::KeyY,
    KeyCode::KeyZ,
];

const DIGIT_KEYS: [KeyCode; 10] = [
    KeyCode::Digit0,
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
];

#[derive(Deserialize, Resource)]
pub struct CreditsConfig {
    pub font_size: f32,
    pub credits_speed: f32,
    pub screen_height: f32,
    #[serde(default = "default_skip_key")]
    pub skip_key: String,
    #[serde(default = "default_skip_hold_seconds")]
    pub skip_hold_seconds: f32,
    pub sections: Vec<CreditsSection>,
}

#[derive(Deserialize)]
pub struct CreditsSection {
    pub header: String,
    pub text: String,
}

fn default_skip_key() -> String {
    "space".to_owned()
}

fn default_skip_hold_seconds() -> f32 {
    3.0
}

/// Parses a human-readable key name (e.g. `"space"`, `"Enter"`, `"f"`, `"3"`) into a [`KeyCode`].
///
/// Returns `None` for empty or unknown names; callers fall back to [`KeyCode::Space`].
pub(crate) fn parse_skip_key(raw: &str) -> Option<KeyCode> {
    let name = raw.trim().to_ascii_lowercase();

    let named = match name.as_str() {
        "space" => Some(KeyCode::Space),
        "enter" | "return" => Some(KeyCode::Enter),
        "escape" | "esc" => Some(KeyCode::Escape),
        "tab" => Some(KeyCode::Tab),
        "backspace" => Some(KeyCode::Backspace),
        "delete" | "del" => Some(KeyCode::Delete),
        "insert" | "ins" => Some(KeyCode::Insert),
        "up" => Some(KeyCode::ArrowUp),
        "down" => Some(KeyCode::ArrowDown),
        "left" => Some(KeyCode::ArrowLeft),
        "right" => Some(KeyCode::ArrowRight),
        "home" => Some(KeyCode::Home),
        "end" => Some(KeyCode::End),
        "pageup" | "page_up" => Some(KeyCode::PageUp),
        "pagedown" | "page_down" => Some(KeyCode::PageDown),
        "shift" => Some(KeyCode::ShiftLeft),
        "ctrl" | "control" => Some(KeyCode::ControlLeft),
        "alt" => Some(KeyCode::AltLeft),
        "f1" => Some(KeyCode::F1),
        "f2" => Some(KeyCode::F2),
        "f3" => Some(KeyCode::F3),
        "f4" => Some(KeyCode::F4),
        "f5" => Some(KeyCode::F5),
        "f6" => Some(KeyCode::F6),
        "f7" => Some(KeyCode::F7),
        "f8" => Some(KeyCode::F8),
        "f9" => Some(KeyCode::F9),
        "f10" => Some(KeyCode::F10),
        "f11" => Some(KeyCode::F11),
        "f12" => Some(KeyCode::F12),
        _ => None,
    };

    if named.is_some() {
        return named;
    }

    let mut chars = name.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_lowercase() => Some(LETTER_KEYS[(c as u8 - b'a') as usize]),
        (Some(c), None) if c.is_ascii_digit() => Some(DIGIT_KEYS[(c as u8 - b'0') as usize]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_named_keys_case_insensitively() {
        assert_eq!(parse_skip_key("Space"), Some(KeyCode::Space));
        assert_eq!(parse_skip_key("  ESC "), Some(KeyCode::Escape));
        assert_eq!(parse_skip_key("PageDown"), Some(KeyCode::PageDown));
        assert_eq!(parse_skip_key("return"), Some(KeyCode::Enter));
    }

    #[test]
    fn parses_letters_and_digits() {
        assert_eq!(parse_skip_key("f"), Some(KeyCode::KeyF));
        assert_eq!(parse_skip_key("A"), Some(KeyCode::KeyA));
        assert_eq!(parse_skip_key("0"), Some(KeyCode::Digit0));
        assert_eq!(parse_skip_key("9"), Some(KeyCode::Digit9));
    }

    #[test]
    fn rejects_unknown_keys() {
        assert_eq!(parse_skip_key(""), None);
        assert_eq!(parse_skip_key("hyper"), None);
        assert_eq!(parse_skip_key("ab"), None);
        assert_eq!(parse_skip_key("!"), None);
    }
}
