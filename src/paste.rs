//! Simulated paste keystroke and the input-simulation permission check.

use std::fmt;

use enigo::{Direction, Enigo, InputError, Key, Keyboard, NewConError, Settings};
use global_hotkey::hotkey::Modifiers;

use crate::hotkey::PRIMARY_MODIFIER;

const PASTE_KEY: char = 'v';
/// System Settings > Privacy & Security > Accessibility. Only opened on macOS.
const ACCESSIBILITY_SETTINGS_URL: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";
const URL_OPENER: &str = "open";
/// Each hotkey modifier with the key that enigo presses for it.
const MODIFIER_KEYS: [(Modifiers, Key); 4] = [
    (Modifiers::SHIFT, Key::Shift),
    (Modifiers::CONTROL, Key::Control),
    (Modifiers::ALT, Key::Alt),
    (Modifiers::META, Key::Meta),
];

#[derive(Debug)]
pub enum PasteError {
    /// macOS Accessibility permission is not granted.
    NoPermission,
    Connect(NewConError),
    Input {
        step: &'static str,
        source: InputError,
    },
    OpenSettings(std::io::Error),
}

impl fmt::Display for PasteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PasteError::NoPermission => {
                write!(
                    f,
                    "paste: expected Accessibility permission, actual not granted"
                )
            }
            PasteError::Connect(e) => write!(f, "paste: input simulation connection failed: {e}"),
            PasteError::Input { step, source } => {
                write!(f, "paste: key event `{step}` failed: {source}")
            }
            PasteError::OpenSettings(e) => {
                write!(
                    f,
                    "paste: opening {ACCESSIBILITY_SETTINGS_URL} with `{URL_OPENER}` failed: {e}"
                )
            }
        }
    }
}

impl std::error::Error for PasteError {}

fn connect() -> Result<Enigo, PasteError> {
    let settings = Settings {
        open_prompt_to_get_permissions: false,
        ..Settings::default()
    };
    Enigo::new(&settings).map_err(|e| match e {
        NewConError::NoPermission => PasteError::NoPermission,
        other => PasteError::Connect(other),
    })
}

/// True when input simulation is allowed. Only macOS can answer false.
pub fn accessibility_granted() -> Result<bool, PasteError> {
    match connect() {
        Ok(_) => Ok(true),
        Err(PasteError::NoPermission) => Ok(false),
        Err(e) => Err(e),
    }
}

pub fn open_accessibility_settings() -> Result<(), PasteError> {
    std::process::Command::new(URL_OPENER)
        .arg(ACCESSIBILITY_SETTINGS_URL)
        .spawn()
        .map(drop)
        .map_err(PasteError::OpenSettings)
}

fn send_key(
    enigo: &mut Enigo,
    key: Key,
    direction: Direction,
    step: &'static str,
) -> Result<(), PasteError> {
    enigo
        .key(key, direction)
        .map_err(|source| PasteError::Input { step, source })
}

/// Releases the hotkey's modifiers, then sends Cmd+V (macOS) or Ctrl+V (Windows, Linux).
pub fn simulate_paste(hotkey_modifiers: Modifiers) -> Result<(), PasteError> {
    let mut enigo = connect()?;
    for (modifier, modifier_key) in MODIFIER_KEYS {
        if hotkey_modifiers.contains(modifier) {
            send_key(
                &mut enigo,
                modifier_key,
                Direction::Release,
                "release hotkey modifier",
            )?;
        }
    }
    let (_, paste_modifier) = MODIFIER_KEYS
        .into_iter()
        .find(|(modifier, _)| *modifier == PRIMARY_MODIFIER)
        .expect("PRIMARY_MODIFIER is one of MODIFIER_KEYS");
    send_key(
        &mut enigo,
        paste_modifier,
        Direction::Press,
        "press paste modifier",
    )?;
    send_key(
        &mut enigo,
        Key::Unicode(PASTE_KEY),
        Direction::Click,
        "click V",
    )?;
    send_key(
        &mut enigo,
        paste_modifier,
        Direction::Release,
        "release paste modifier",
    )
}
