mod app;

use std::process::ExitCode;

use plainpaste::{clipboard, config, hotkey, logging};

slint::include_modules!();

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let settings_path = config::settings_path()?;
    let settings = config::load_or_create(&settings_path)?;
    log::info!("settings loaded from {}", settings_path.display());

    app::flyout::select_tray_only_backend()?;
    let ui = Flyout::new()?;
    let clipboard = clipboard::PlatformClipboard::new()?;
    let hotkeys = hotkey::HotkeyService::new(settings.hotkey)?;
    let (events, receiver) = app::events::channel();
    app::events::connect_sources(&ui, &events);
    app::flyout::install_focus_hook(&ui, events.clone());

    let app = app::App::new(
        settings,
        settings_path,
        clipboard,
        app::SlintTimer,
        hotkeys,
        events.clone(),
        ui,
    );
    app::spawn_event_pump(app, receiver, events)?;
    slint::run_event_loop_until_quit()?;
    Ok(())
}

fn main() -> ExitCode {
    if let Err(e) = logging::init() {
        eprintln!("PlainPaste stopped: {e}");
        return ExitCode::FAILURE;
    }
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            log::error!("PlainPaste stopped: {e}");
            ExitCode::FAILURE
        }
    }
}
