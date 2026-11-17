//! Tray icon and menu. Events are forwarded to the app event channel.

use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use super::events::{AppEvent, EventSender};

const TOOLTIP: &str = "PlainPaste";
const MENU_SETTINGS: &str = "設定";
const MENU_QUIT: &str = "結束";
/// macOS: Fluent clipboard + sparkle glyph, black + alpha, drawn as a template image (64 px, shown at 18 pt).
/// Other OSes: the color app icon, because a black glyph is invisible on dark taskbars.
/// Both are raw straight-alpha RGBA rendered with sips from assets/app-icon.svg and design/tray.svg by scripts/gen_tray_assets.py.
const ICON_MACOS: &[u8] = include_bytes!("../../assets/tray-macos-64.rgba");
const ICON_MACOS_SIZE: u32 = 64;
const ICON_COLOR: &[u8] = include_bytes!("../../assets/tray-color-32.rgba");
const ICON_COLOR_SIZE: u32 = 32;
const IS_MACOS: bool = cfg!(target_os = "macos");

/// Keeps the tray icon alive. Dropping it removes the icon.
pub struct Tray {
    _icon: TrayIcon,
}

#[derive(Debug)]
pub enum TrayError {
    Icon(tray_icon::BadIcon),
    Menu(tray_icon::menu::Error),
    Build(tray_icon::Error),
}

impl std::fmt::Display for TrayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrayError::Icon(e) => write!(f, "tray: icon RGBA rejected: {e}"),
            TrayError::Menu(e) => write!(f, "tray: menu creation failed: {e}"),
            TrayError::Build(e) => write!(f, "tray: icon creation failed: {e}"),
        }
    }
}

impl std::error::Error for TrayError {}

fn icon() -> Result<Icon, TrayError> {
    let (rgba, size) = if IS_MACOS {
        (ICON_MACOS, ICON_MACOS_SIZE)
    } else {
        (ICON_COLOR, ICON_COLOR_SIZE)
    };
    Icon::from_rgba(rgba.to_vec(), size, size).map_err(TrayError::Icon)
}

/// Builds the tray. Call it on the main thread after the event loop has started (macOS requirement).
/// A left click sends `OpenFlyout` with the icon rect. The menu has 設定 and 結束.
pub fn build(events: EventSender) -> Result<Tray, TrayError> {
    let settings = MenuItem::new(MENU_SETTINGS, true, None);
    let quit = MenuItem::new(MENU_QUIT, true, None);
    let menu = Menu::with_items(&[&settings, &quit]).map_err(TrayError::Menu)?;
    let settings_id: MenuId = settings.id().clone();
    let quit_id: MenuId = quit.id().clone();

    let menu_events = events.clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        if event.id == settings_id {
            menu_events.send(AppEvent::OpenFlyout { anchor: None });
        } else if event.id == quit_id {
            menu_events.send(AppEvent::Quit);
        }
    }));
    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        if let TrayIconEvent::Click {
            rect,
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event
        {
            events.send(AppEvent::OpenFlyout { anchor: Some(rect) });
        }
    }));

    let builder = TrayIconBuilder::new()
        .with_tooltip(TOOLTIP)
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false);
    // tray-icon 0.26 only defines `with_icon_templated` on macOS (the cross-platform flag is deprecated).
    #[cfg(target_os = "macos")]
    let builder = builder.with_icon_templated(icon()?);
    #[cfg(not(target_os = "macos"))]
    let builder = builder.with_icon(icon()?);
    let icon = builder.build().map_err(TrayError::Build)?;
    Ok(Tray { _icon: icon })
}
