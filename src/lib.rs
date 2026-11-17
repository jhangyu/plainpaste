//! Platform divergence policy: code is shared across macOS, Windows and Linux.
//! Only these places may branch on `target_os`, each behind one named item:
//! - `clipboard`: one native backend per OS (pasteboard, Win32, X11).
//! - `hotkey::PRIMARY_MODIFIER`: Cmd on macOS, Ctrl elsewhere. The default
//!   hotkey, the key recorder and the paste key derive from it.
//! - `app::recorder` keycap labels: the Super key reads "Cmd" or "Win".
//! - `config::config_dir` and `config::log_dir`: one path rule per OS; macOS logs go to the standard `~/Library/Logs` folder.
//! - `app::tray`: macOS uses a template (monochrome) tray icon.
//! - `app::flyout`: taskbar hiding differs per windowing system.
//! - `app`: the accessibility permission row is shown on macOS only.

pub mod clipboard;
pub mod config;
pub mod hotkey;
pub mod logging;
pub mod paste;
pub mod transform;
