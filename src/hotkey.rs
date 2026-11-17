//! Global paste hotkey: registration and replacement.

use global_hotkey::hotkey::{HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyManager, HotKeyState};

pub use global_hotkey::Error as HotkeyError;

/// The modifier of the platform's primary shortcut key: Cmd on macOS, Ctrl elsewhere.
/// The one place this decision lives.
pub const PRIMARY_MODIFIER: Modifiers = if cfg!(target_os = "macos") {
    Modifiers::META
} else {
    Modifiers::CONTROL
};

/// Owns the OS registration. At most one hotkey is registered at a time.
pub struct HotkeyService {
    manager: GlobalHotKeyManager,
    registered: Option<HotKey>,
}

impl HotkeyService {
    pub fn new(initial: HotKey) -> Result<Self, HotkeyError> {
        let manager = GlobalHotKeyManager::new()?;
        manager.register(initial)?;
        Ok(HotkeyService {
            manager,
            registered: Some(initial),
        })
    }

    /// True when `id`/`state` is the Released event of the registered hotkey.
    pub fn matches_release_event(&self, id: u32, state: HotKeyState) -> bool {
        state == HotKeyState::Released && self.registered.is_some_and(|h| h.id() == id)
    }

    /// Unregisters the current hotkey, so the recorder can capture its keys.
    pub fn suspend(&mut self) -> Result<(), HotkeyError> {
        if let Some(current) = self.registered.take() {
            self.manager.unregister(current)?;
        }
        Ok(())
    }

    /// Registers `hotkey`. On failure nothing is registered.
    pub fn register(&mut self, hotkey: HotKey) -> Result<(), HotkeyError> {
        self.suspend()?;
        self.manager.register(hotkey)?;
        self.registered = Some(hotkey);
        Ok(())
    }
}
