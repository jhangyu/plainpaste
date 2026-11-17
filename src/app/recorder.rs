//! Hotkey recorder: maps a UI key press to a hotkey and a hotkey to keycap labels.

use global_hotkey::hotkey::{HotKey, Modifiers};
use plainpaste::hotkey::PRIMARY_MODIFIER;

const DIGITS: &str = "0123456789";
const FUNCTION_KEY_COUNT: u8 = 12;
const SPACE: &str = " ";
const LABEL_SUPER_MAC: &str = "Cmd";
const LABEL_SUPER_OTHER: &str = "Win";

/// Slint's KeyboardModifiers of a key press. On macOS Slint reports Command as `control`
/// and Control as `meta`. global-hotkey calls Command META.
#[derive(Debug, Clone, Copy)]
pub struct KeyModifiers {
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

/// Converts a recorder key press into a hotkey.
/// Returns `None` when there is no modifier or the key has no hotkey code.
pub fn hotkey_from_key_press(text: &str, modifiers: KeyModifiers) -> Option<HotKey> {
    let control_or_super = (Modifiers::META | Modifiers::CONTROL) - PRIMARY_MODIFIER;
    let mut mods = Modifiers::empty();
    if modifiers.control {
        mods |= PRIMARY_MODIFIER;
    }
    if modifiers.meta {
        mods |= control_or_super;
    }
    if modifiers.alt {
        mods |= Modifiers::ALT;
    }
    if modifiers.shift {
        mods |= Modifiers::SHIFT;
    }
    if mods.is_empty() {
        return None;
    }
    // A single token parses as a hotkey without modifiers. Only its key code is used.
    let key = key_token(text)?.parse::<HotKey>().ok()?.key;
    Some(HotKey::new(Some(mods), key))
}

/// global-hotkey key token for a Slint key text, for example "v" -> "KeyV", F5 -> "F5".
fn key_token(text: &str) -> Option<String> {
    let mut chars = text.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    if c.is_ascii_alphabetic() {
        return Some(format!("Key{}", c.to_ascii_uppercase()));
    }
    if DIGITS.contains(c) {
        return Some(format!("Digit{c}"));
    }
    if text == SPACE {
        return Some("Space".to_string());
    }
    (1..=FUNCTION_KEY_COUNT)
        .zip(FUNCTION_KEYS)
        .find(|(_, key)| char::from(*key) == c)
        .map(|(n, _)| format!("F{n}"))
}

const FUNCTION_KEYS: [slint::platform::Key; FUNCTION_KEY_COUNT as usize] = [
    slint::platform::Key::F1,
    slint::platform::Key::F2,
    slint::platform::Key::F3,
    slint::platform::Key::F4,
    slint::platform::Key::F5,
    slint::platform::Key::F6,
    slint::platform::Key::F7,
    slint::platform::Key::F8,
    slint::platform::Key::F9,
    slint::platform::Key::F10,
    slint::platform::Key::F11,
    slint::platform::Key::F12,
];

/// Keycap labels in display order, for example ["Ctrl", "Shift", "V"].
pub fn keycap_labels(hotkey: HotKey) -> Vec<String> {
    let super_label = if cfg!(target_os = "macos") {
        LABEL_SUPER_MAC
    } else {
        LABEL_SUPER_OTHER
    };
    let mut labels: Vec<String> = [
        (Modifiers::META, super_label),
        (Modifiers::CONTROL, "Ctrl"),
        (Modifiers::ALT, "Alt"),
        (Modifiers::SHIFT, "Shift"),
    ]
    .into_iter()
    .filter(|(m, _)| hotkey.mods.contains(*m))
    .map(|(_, label)| label.to_string())
    .collect();
    let code = hotkey.key.to_string();
    let key = code
        .strip_prefix("Key")
        .or_else(|| code.strip_prefix("Digit"))
        .unwrap_or(&code)
        .to_string();
    labels.push(key);
    labels
}
