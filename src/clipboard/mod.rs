//! One clipboard interface, one native implementation per OS.
//! The crate doc in src/lib.rs states the platform divergence policy.

use std::fmt;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
/// The X11 implementation. Wayland-only sessions are not supported.
mod x11;

#[cfg(target_os = "macos")]
pub use macos::MacosClipboard as PlatformClipboard;
#[cfg(target_os = "windows")]
pub use windows::WindowsClipboard as PlatformClipboard;
#[cfg(target_os = "linux")]
pub use x11::X11Clipboard as PlatformClipboard;

#[derive(Debug)]
pub struct ClipboardError {
    /// The clipboard operation that failed, for example "snapshot".
    pub op: &'static str,
    /// Platform API name, error code and the affected format or item.
    pub detail: String,
}

impl ClipboardError {
    pub fn new(op: &'static str, detail: impl Into<String>) -> Self {
        ClipboardError {
            op,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "clipboard {} failed: {}", self.op, self.detail)
    }
}

impl std::error::Error for ClipboardError {}

pub trait Clipboard {
    /// Every format that was on the clipboard, with its bytes.
    type Snapshot;

    /// The plain-text representation, or `None` when the clipboard has no text.
    fn read_text(&mut self) -> Result<Option<String>, ClipboardError>;

    /// Replaces the clipboard content with `text` as plain text.
    fn write_text(&mut self, text: &str) -> Result<(), ClipboardError>;

    /// Copies every format currently on the clipboard.
    fn snapshot(&mut self) -> Result<Self::Snapshot, ClipboardError>;

    /// Replaces the clipboard content with a lazy plain-text offer.
    /// The OS asks for the bytes only when a target reads them.
    fn offer_lazy_text(&mut self, text: String) -> Result<(), ClipboardError>;

    /// True when the clipboard still holds the offer made by the last `offer_lazy_text`
    /// (no other app has written to it since).
    fn offer_is_current(&mut self) -> Result<bool, ClipboardError>;

    /// Replaces the clipboard content with `snapshot`.
    fn restore(&mut self, snapshot: Self::Snapshot) -> Result<(), ClipboardError>;
}
