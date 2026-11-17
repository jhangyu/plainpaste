//! App state and the single event handler. Every event source (hotkey, tray, menu,
//! clipboard provider, one-shot timer, UI) sends an `AppEvent`. One local task on
//! the Slint event loop receives them in order, so handlers never nest.

use std::path::PathBuf;
use std::time::Duration;

use global_hotkey::hotkey::HotKey;
use plainpaste::clipboard::{Clipboard, ClipboardError};
use plainpaste::config::{self, Settings};
use plainpaste::hotkey::{self, HotkeyService};
use plainpaste::paste;
use plainpaste::transform::{LINE_BREAK_REPLACEMENT, normalize_line_breaks};
use slint::{ModelRc, SharedString, VecModel};

use crate::Flyout;

pub mod events;
pub mod flyout;
mod recorder;
mod restore;
mod tray;

use events::{AppEvent, EventReceiver, EventSender};
use recorder::{hotkey_from_key_press, keycap_labels};
use restore::RestoreSchedule;

/// Fail-fast bound: if no app reads the lazy text within this time, restore the snapshot.
pub const PASTE_READ_TIMEOUT: Duration = Duration::from_secs(2);
const PREVIEW_SOURCE: &str = "一行\n二行\n三行";
const PREVIEW_LINE_BREAK: char = '\n';
const HOTKEY_TAKEN_MESSAGE: &str = "已被其他程式使用";
const HOTKEY_INVALID_MESSAGE: &str =
    "無效的組合：至少需一個修飾鍵，按鍵限字母、數字、空白或 F1–F12。";

/// Injectable clock for the paste-read timeout.
pub trait OneShotTimer {
    /// Calls `on_expire` once, `after` from now, on the event-loop thread.
    fn schedule(&self, after: Duration, on_expire: Box<dyn FnOnce()>);
}

/// Production clock: a Slint single-shot timer.
pub struct SlintTimer;

impl OneShotTimer for SlintTimer {
    fn schedule(&self, after: Duration, on_expire: Box<dyn FnOnce()>) {
        slint::Timer::single_shot(after, on_expire);
    }
}

#[derive(Debug)]
enum PasteFlowError {
    Clipboard(ClipboardError),
    Paste(paste::PasteError),
}

impl From<ClipboardError> for PasteFlowError {
    fn from(e: ClipboardError) -> Self {
        PasteFlowError::Clipboard(e)
    }
}

impl From<paste::PasteError> for PasteFlowError {
    fn from(e: paste::PasteError) -> Self {
        PasteFlowError::Paste(e)
    }
}

impl std::fmt::Display for PasteFlowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PasteFlowError::Clipboard(e) => write!(f, "{e}"),
            PasteFlowError::Paste(e) => write!(f, "{e}"),
        }
    }
}

pub struct App<C: Clipboard, T: OneShotTimer> {
    settings: Settings,
    settings_path: PathBuf,
    clipboard: C,
    timer: T,
    hotkeys: HotkeyService,
    events: EventSender,
    ui: Flyout,
    restore: RestoreSchedule<C::Snapshot>,
}

impl<C: Clipboard, T: OneShotTimer> App<C, T> {
    pub fn new(
        settings: Settings,
        settings_path: PathBuf,
        clipboard: C,
        timer: T,
        hotkeys: HotkeyService,
        events: EventSender,
        ui: Flyout,
    ) -> Self {
        let restore = RestoreSchedule::new(settings.restore_clipboard);
        let app = App {
            settings,
            settings_path,
            clipboard,
            timer,
            hotkeys,
            events,
            ui,
            restore,
        };
        app.ui
            .set_version(SharedString::from(env!("CARGO_PKG_VERSION")));
        app.ui.set_show_accessibility(cfg!(target_os = "macos"));
        app.sync_ui();
        app
    }

    pub fn handle(&mut self, event: AppEvent) {
        match event {
            AppEvent::Hotkey { id, state } => {
                if self.hotkeys.matches_release_event(id, state)
                    && let Err(e) = self.paste_flow()
                {
                    log::error!("paste aborted: {e}");
                }
            }
            AppEvent::OpenFlyout { anchor } => self.open_flyout(anchor),
            AppEvent::FlyoutFocusLost => {
                if self.ui.get_recording() {
                    self.cancel_recording();
                }
                flyout::hide(&self.ui);
            }
            AppEvent::PasteReadTimeout { session } => self.finish_timed_out(session),
            AppEvent::StartRecording => self.start_recording(),
            AppEvent::CancelRecording => self.cancel_recording(),
            AppEvent::RecordKey { text, modifiers } => {
                match hotkey_from_key_press(&text, modifiers) {
                    Some(new) => self.apply_hotkey(new),
                    None => self.ui.set_hotkey_error(HOTKEY_INVALID_MESSAGE.into()),
                }
            }
            AppEvent::ResetHotkey => self.apply_hotkey(config::default_hotkey()),
            AppEvent::ToggleKeepLineBreaks => {
                self.settings.keep_line_breaks = !self.settings.keep_line_breaks;
                self.save_and_sync();
            }
            AppEvent::ToggleRestoreClipboard => {
                self.settings.restore_clipboard = self.restore.toggle();
                self.save_and_sync();
            }
            AppEvent::ChooseTheme(theme) => {
                self.settings.theme = theme;
                self.save_and_sync();
            }
            AppEvent::OpenAccessibilitySettings => {
                if let Err(e) = paste::open_accessibility_settings() {
                    log::error!("{e}");
                }
            }
            AppEvent::Quit => self.quit(),
        }
    }

    fn paste_flow(&mut self) -> Result<(), PasteFlowError> {
        if !paste::accessibility_granted()? {
            log::warn!("paste skipped: Accessibility permission not granted");
            return Ok(());
        }
        let Some(text) = self.clipboard.read_text()? else {
            log::info!("paste skipped: clipboard has no plain text, nothing changed");
            return Ok(());
        };
        let out = normalize_line_breaks(&text, self.settings.keep_line_breaks);
        let modifiers = self.settings.hotkey.mods;
        if !self.restore.enabled() {
            self.clipboard.write_text(&out)?;
            return Ok(paste::simulate_paste(modifiers)?);
        }
        if let Some(previous) = self.restore.take() {
            self.restore(previous, "a new paste started before the previous restore");
        }
        let snapshot = self.clipboard.snapshot()?;
        if let Err(e) = self.clipboard.offer_lazy_text(out) {
            self.restore(snapshot, "offering the lazy text failed");
            return Err(e.into());
        }
        if let Err(e) = paste::simulate_paste(modifiers) {
            self.restore(snapshot, "simulating the paste failed");
            return Err(e.into());
        }
        let session = self.restore.schedule(snapshot);
        let timeout_events = self.events.clone();
        self.timer.schedule(
            PASTE_READ_TIMEOUT,
            Box::new(move || timeout_events.send(AppEvent::PasteReadTimeout { session })),
        );
        Ok(())
    }

    /// At the read deadline: restores the pending snapshot only if our offer is still on the
    /// clipboard. If another app wrote to it since, the snapshot is dropped.
    fn finish_timed_out(&mut self, session: u64) {
        let Some(snapshot) = self.restore.take_due(session) else {
            return;
        };
        match self.clipboard.offer_is_current() {
            Ok(true) => self.restore(snapshot, "read deadline reached"),
            Ok(false) => log::info!("clipboard changed by another app, restore skipped"),
            Err(e) => log::error!("clipboard restore skipped: offer check failed: {e}"),
        }
    }

    fn restore(&mut self, snapshot: C::Snapshot, reason: &str) {
        match self.clipboard.restore(snapshot) {
            Ok(()) => log::info!("clipboard restored: {reason}"),
            Err(e) => log::error!("clipboard restore after `{reason}` failed: {e}"),
        }
    }

    fn open_flyout(&mut self, anchor: Option<tray_icon::Rect>) {
        match paste::accessibility_granted() {
            Ok(granted) => self.ui.set_accessibility_granted(granted),
            Err(e) => log::error!("accessibility check failed: {e}"),
        }
        flyout::show(&self.ui, anchor);
    }

    fn start_recording(&mut self) {
        if let Err(e) = self.hotkeys.suspend() {
            log::error!("hotkey recording: unregister failed: {e}");
            self.ui.set_hotkey_error(SharedString::from(e.to_string()));
            return;
        }
        self.ui.set_hotkey_error(SharedString::new());
        self.ui.set_recording(true);
    }

    fn cancel_recording(&mut self) {
        self.ui.set_recording(false);
        if let Err(e) = self.hotkeys.register(self.settings.hotkey) {
            log::error!("hotkey re-register of {} failed: {e}", self.settings.hotkey);
            self.ui.set_hotkey_error(SharedString::from(e.to_string()));
        }
    }

    /// Registers `new`. On failure the previous hotkey is registered again.
    fn apply_hotkey(&mut self, new: HotKey) {
        self.ui.set_recording(false);
        match self.hotkeys.register(new) {
            Ok(()) => {
                self.settings.hotkey = new;
                self.ui.set_hotkey_error(SharedString::new());
                self.save_and_sync();
            }
            Err(e) => {
                log::warn!("hotkey {new} registration failed: {e}");
                let message = match e {
                    hotkey::HotkeyError::AlreadyRegistered(_) => HOTKEY_TAKEN_MESSAGE.to_string(),
                    other => other.to_string(),
                };
                self.ui.set_hotkey_error(SharedString::from(message));
                if let Err(e) = self.hotkeys.register(self.settings.hotkey) {
                    log::error!("hotkey re-register of {} failed: {e}", self.settings.hotkey);
                }
            }
        }
    }

    fn save_and_sync(&mut self) {
        if let Err(e) = config::save(&self.settings_path, &self.settings) {
            log::error!("{e}");
        }
        self.sync_ui();
    }

    fn sync_ui(&self) {
        let ui = &self.ui;
        let caps: Vec<SharedString> = keycap_labels(self.settings.hotkey)
            .into_iter()
            .map(SharedString::from)
            .collect();
        ui.set_hotkey_caps(ModelRc::new(VecModel::from(caps)));
        ui.set_keep_line_breaks(self.settings.keep_line_breaks);
        ui.set_restore_clipboard(self.settings.restore_clipboard);
        let preview_out = normalize_line_breaks(PREVIEW_SOURCE, self.settings.keep_line_breaks);
        let separator = if self.settings.keep_line_breaks {
            PREVIEW_LINE_BREAK
        } else {
            LINE_BREAK_REPLACEMENT
        };
        ui.set_preview_in(string_model(PREVIEW_SOURCE.split(PREVIEW_LINE_BREAK)));
        ui.set_preview_out(string_model(preview_out.split(separator)));
        ui.set_theme(self.settings.theme.into());
        flyout::apply_theme(ui, self.settings.theme);
    }

    fn quit(&mut self) {
        if let Some(snapshot) = self.restore.take() {
            self.restore(snapshot, "the app is quitting");
        }
        if let Err(e) = slint::quit_event_loop() {
            log::error!("quit: event loop already gone: {e}");
        }
    }
}

fn string_model<'a>(parts: impl Iterator<Item = &'a str>) -> ModelRc<SharedString> {
    ModelRc::new(VecModel::from(
        parts.map(SharedString::from).collect::<Vec<_>>(),
    ))
}

/// Runs the app: one local task receives every event in order.
pub fn spawn_event_pump<C: Clipboard + 'static, T: OneShotTimer + 'static>(
    app: App<C, T>,
    mut events: EventReceiver,
    tray_events: EventSender,
) -> Result<(), slint::EventLoopError> {
    let mut app = app;
    slint::spawn_local(async move {
        // macOS: the tray must be created after the event loop has started. This task runs then.
        let _tray = match tray::build(tray_events) {
            Ok(tray) => tray,
            Err(e) => {
                log::error!("{e}");
                if let Err(e) = slint::quit_event_loop() {
                    log::error!("quit: {e}");
                }
                return;
            }
        };
        loop {
            app.handle(events.recv().await);
        }
    })
    .map(drop)
}
