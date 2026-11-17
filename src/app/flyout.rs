//! Flyout window placement, focus-loss closing and theme switching.

use plainpaste::config::Theme;
use slint::language::ColorScheme;
use slint::winit_030::winit::dpi::{PhysicalPosition as WinitPosition, PhysicalSize as WinitSize};
use slint::winit_030::winit::event::WindowEvent;
use slint::winit_030::{EventResult, WinitWindowAccessor};
use slint::{ComponentHandle, PhysicalPosition, PhysicalSize};

use super::events::{AppEvent, EventSender};
use crate::{Flyout, Palette, ThemeChoice, Tokens};

const HALF: f64 = 2.0;

/// A monitor's area in physical pixels.
#[derive(Debug, Clone, Copy)]
struct MonitorArea {
    pub position: WinitPosition<i32>,
    pub size: WinitSize<u32>,
}

/// Where the flyout goes relative to the tray icon.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Placement {
    pub position: PhysicalPosition,
    /// True when the flyout hangs below the icon, so the caret points up at it.
    pub caret_visible: bool,
}

/// Places the flyout next to the tray icon: below it when the icon is in the top half
/// of its monitor (macOS menu bar), above it otherwise (Windows taskbar).
/// The caret sits over the icon center, `caret_from_right` logical px from the flyout's right edge.
/// The result is clamped to the monitor horizontally.
fn anchored_placement(
    icon: tray_icon::Rect,
    flyout: PhysicalSize,
    scale: f32,
    caret_from_right: f32,
    monitor: MonitorArea,
) -> Placement {
    let icon_center_x = icon.position.x + f64::from(icon.size.width) / HALF;
    let caret_from_right = f64::from(caret_from_right * scale);
    let monitor_left = f64::from(monitor.position.x);
    let monitor_right = monitor_left + f64::from(monitor.size.width) - f64::from(flyout.width);
    let x = (icon_center_x - f64::from(flyout.width) + caret_from_right)
        .clamp(monitor_left, monitor_right.max(monitor_left));
    let monitor_middle_y = f64::from(monitor.position.y) + f64::from(monitor.size.height) / HALF;
    let below = icon.position.y < monitor_middle_y;
    let y = if below {
        icon.position.y + f64::from(icon.size.height)
    } else {
        icon.position.y - f64::from(flyout.height)
    };
    Placement {
        position: PhysicalPosition::new(x.round() as i32, y.round() as i32),
        caret_visible: below,
    }
}

/// Centers the flyout on the monitor (Linux: AppIndicator/KSNI give no icon position).
fn centered_position(flyout: PhysicalSize, monitor: MonitorArea) -> PhysicalPosition {
    let x = i64::from(monitor.position.x)
        + (i64::from(monitor.size.width) - i64::from(flyout.width)) / 2;
    let y = i64::from(monitor.position.y)
        + (i64::from(monitor.size.height) - i64::from(flyout.height)) / 2;
    PhysicalPosition::new(x as i32, y as i32)
}

impl From<Theme> for ThemeChoice {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::System => ThemeChoice::System,
            Theme::Light => ThemeChoice::Light,
            Theme::Dark => ThemeChoice::Dark,
        }
    }
}

impl From<ThemeChoice> for Theme {
    fn from(choice: ThemeChoice) -> Self {
        match choice {
            ThemeChoice::System => Theme::System,
            ThemeChoice::Light => Theme::Light,
            ThemeChoice::Dark => Theme::Dark,
        }
    }
}

/// `System` uses `ColorScheme::Unknown`, which follows the OS.
fn color_scheme(theme: Theme) -> ColorScheme {
    match theme {
        Theme::System => ColorScheme::Unknown,
        Theme::Light => ColorScheme::Light,
        Theme::Dark => ColorScheme::Dark,
    }
}

/// Switches the Fluent palette.
pub fn apply_theme(ui: &Flyout, theme: Theme) {
    ui.global::<Palette>().set_color_scheme(color_scheme(theme));
}

/// Selects Slint's winit backend so the app is tray-only on every OS (no Dock icon, taskbar button or
/// Cmd+Tab entry). Must run before the first window is created.
pub fn select_tray_only_backend() -> Result<(), slint::PlatformError> {
    use slint::winit_030::winit::event_loop::EventLoop;
    let event_loop = EventLoop::with_user_event();
    // macOS: Accessory is set at runtime, so it also holds for an unbundled `cargo run`.
    // `LSUIElement` in Info.plist (bundle only) additionally avoids the Dock icon flashing at launch.
    #[cfg(target_os = "macos")]
    let event_loop = {
        use slint::winit_030::winit::platform::macos::{
            ActivationPolicy, EventLoopBuilderExtMacOS,
        };
        let mut event_loop = event_loop;
        event_loop.with_activation_policy(ActivationPolicy::Accessory);
        event_loop
    };
    let backend = slint::BackendSelector::new()
        .with_winit_event_loop_builder(event_loop)
        .with_winit_window_attributes_hook(hide_from_taskbar);
    backend.select()
}

/// Keeps the flyout out of the taskbar. Windows: `WS_EX_TOOLWINDOW` via winit's skip-taskbar attribute.
/// X11: winit has no skip-taskbar attribute, so the window type is Utility, which taskbars skip.
/// macOS needs nothing here (app-level Accessory policy); Wayland has no taskbar-skip protocol.
/// `#[cfg]` is required: each platform extension trait exists only on its OS.
fn hide_from_taskbar(
    attributes: slint::winit_030::winit::window::WindowAttributes,
) -> slint::winit_030::winit::window::WindowAttributes {
    #[cfg(target_os = "windows")]
    let attributes = {
        use slint::winit_030::winit::platform::windows::WindowAttributesExtWindows;
        attributes.with_skip_taskbar(true)
    };
    #[cfg(target_os = "linux")]
    let attributes = {
        use slint::winit_030::winit::platform::x11::{WindowAttributesExtX11, WindowType};
        attributes.with_x11_window_type(vec![WindowType::Utility])
    };
    attributes
}

/// Installs the focus-loss hook. The hook only sends an event, so it never re-enters app state.
pub fn install_focus_hook(ui: &Flyout, events: EventSender) {
    ui.window().on_winit_window_event(move |_, event| {
        if let WindowEvent::Focused(false) = event {
            events.send(AppEvent::FlyoutFocusLost);
        }
        EventResult::Propagate
    });
}

/// The monitor holding the anchor, else the window's current monitor, else the primary one.
fn monitor_for(
    window: &slint::winit_030::winit::window::Window,
    anchor: Option<tray_icon::Rect>,
) -> Option<MonitorArea> {
    let containing = anchor.and_then(|r| {
        let (x, y) = (r.position.x, r.position.y);
        window.available_monitors().find(|m| {
            let (p, s) = (m.position(), m.size());
            x >= f64::from(p.x)
                && x < f64::from(p.x) + f64::from(s.width)
                && y >= f64::from(p.y)
                && y < f64::from(p.y) + f64::from(s.height)
        })
    });
    containing
        .or_else(|| window.current_monitor())
        .or_else(|| window.primary_monitor())
        .map(|m| MonitorArea {
            position: m.position(),
            size: m.size(),
        })
}

/// Shows the flyout anchored to the tray icon, or centered when there is no usable anchor, then focuses it.
pub fn show(ui: &Flyout, anchor: Option<tray_icon::Rect>) {
    if let Err(e) = ui.show() {
        log::error!("flyout show failed: {e}");
        return;
    }
    let window = ui.window();
    let size = window.size();
    let scale = window.scale_factor();
    // KSNI reports an all-zero rect: treat it as "no anchor".
    let anchor = anchor.filter(|r| r.size.width > 0 && r.size.height > 0);
    let monitor = window.with_winit_window(|w| monitor_for(w, anchor));
    let Some(Some(monitor)) = monitor else {
        log::error!(
            "flyout placement: expected a monitor from winit, actual none. The flyout stays at its last position."
        );
        focus(ui);
        return;
    };
    match anchor {
        Some(icon) => {
            let tokens = ui.global::<Tokens>();
            let caret_from_right = tokens.get_caret_right() + tokens.get_caret() / 2.0;
            let placement = anchored_placement(icon, size, scale, caret_from_right, monitor);
            ui.set_caret_visible(placement.caret_visible);
            window.set_position(placement.position);
        }
        None => {
            ui.set_caret_visible(false);
            window.set_position(centered_position(size, monitor));
        }
    }
    focus(ui);
}

fn focus(ui: &Flyout) {
    if ui
        .window()
        .with_winit_window(|w| w.focus_window())
        .is_none()
    {
        log::error!("flyout focus: expected a winit window, actual none");
    }
}

pub fn hide(ui: &Flyout) {
    if let Err(e) = ui.hide() {
        log::error!("flyout hide failed: {e}");
    }
}
