# PlainPaste — Design Spec

## Goal

PlainPaste is a tray-resident utility for macOS, Windows and Linux (X11). When the user presses a global hotkey, it reads the clipboard as plain text, transforms the text, and pastes it into the focused app. Optionally, it then restores the original clipboard content with all its formats.

## Scope

In scope:
- Global paste hotkey, configurable in the UI.
- "Keep line breaks" setting.
- "Restore clipboard after paste" setting.
- Theme setting: follow system, light or dark.
- Tray icon with a settings flyout. The flyout follows `design/mockups/variant-b.html`.
- macOS Accessibility permission status and a shortcut to System Settings.

Out of scope:
- Linux Wayland sessions.
- Launch at login.
- Clipboard history.
- Background clipboard monitoring. Conversion happens only on the hotkey.
- Any transform other than line-break handling.

## Stack

- A single Rust binary.
- The UI uses Slint with the `fluent` style.
- Crate versions are verified against the registry and the docs during planning.

## Modules

| Module | Responsibility | Dependency |
|---|---|---|
| `config` | Load and save `Settings`. Parse once at the boundary into typed values. | `serde`, `directories` |
| `transform` | Pure function `fn normalize_line_breaks(text: &str, keep_line_breaks: bool) -> String`. | std |
| `clipboard` | One interface with platform-native implementations. It reads plain text, snapshots and restores all formats, and provides data lazily with a "data was read" callback. | `objc2` (macOS), `windows` (Windows), `x11rb` (Linux) |
| `paste` | Release the hotkey's modifiers, then simulate Cmd+V on macOS or Ctrl+V on Windows and Linux. | `enigo` |
| `hotkey` | Register and replace the global hotkey. Emit events, with no polling. | `global-hotkey` |
| `tray` | Tray icon, menu and click events. | `tray-icon` |
| `ui` | The Slint flyout and two-way binding to `Settings`. | `slint` |

The clipboard implementation must differ per platform. Lazy data provision and a full snapshot of all formats exist only in each OS's native clipboard API. `arboard` supports only text, HTML, RGBA images and file lists, and it has no lazy provision.

## Settings

```
Settings {
  hotkey: Hotkey            // default Ctrl+Shift+V (Windows/Linux), Cmd+Shift+V (macOS)
  keep_line_breaks: bool    // default false
  restore_clipboard: bool   // default true
  theme: System | Light | Dark  // default System
}
```

- The settings are stored as JSON in the platform's config directory.
- A file that fails to parse stops startup with an error. The error names the file path, the field, the expected value and the actual value.
- The file is never overwritten silently.

## Paste flow

The hotkey Released event triggers the flow. This event, not Pressed, makes sure the user's keys do not mix with the simulated paste.

1. Read the plain-text representation of the clipboard. If the clipboard has no text, stop. Change nothing and log the reason.
2. `out = normalize_line_breaks(text, keep_line_breaks)`. Line breaks are first normalized to `\n`. If `keep_line_breaks` is false, each run of `\r\n`, `\r` or `\n` becomes one space.
3. If `restore_clipboard` is false, write `out` to the clipboard, then paste. Done.
4. If `restore_clipboard` is true:
   1. Snapshot every format currently on the clipboard.
   2. Offer `out` as plain text through lazy provision:
      - macOS: `NSPasteboardItemDataProvider`.
      - Windows: delayed rendering with `WM_RENDERFORMAT`.
      - X11: answer `SelectionRequest` as the selection owner.
   3. Simulate the paste.
   4. When the OS asks the provider for the data, give `out`. Then queue the restore of the snapshot on the next event-loop turn. The restore cannot happen inside the provider callback.
   5. Fail-fast upper bound `PASTE_READ_TIMEOUT` = 2 s: if nobody requests the data, restore the snapshot and log "paste target never read the clipboard". This is an IO timeout, not a state check. The clock is injectable.

## Tray and flyout

- **macOS and Windows:** a click on the tray icon opens the flyout. The flyout is anchored to the icon rect that the tray click event reports.
- **Linux:** AppIndicator gives no click position. The tray menu has a "設定" item, and that item opens the flyout as a normal centered window.
- **Close:** the flyout closes when it loses focus. "結束" quits the app.
- **Theme:** the theme setting switches the Slint palette color scheme at runtime.

## Error handling

- **Hotkey registration fails** (for example, another app owns the combo): the recorder shows "已被其他程式使用" and the previous hotkey stays registered.
- **macOS Accessibility not granted:** the paste step is skipped and the flyout shows the not-granted state. The status is re-checked each time the flyout opens.
- **Clipboard API errors:** they are logged with context (operation, platform error code) and they abort the current paste. Nothing is swallowed.

## Known limitations

- **Clipboard managers:** Windows clipboard history and macOS clipboard managers may read the lazy data before the target app does. The restore then happens early, and the target may paste the original content.
- **Linux:** only X11 is supported. Under Wayland, global hotkeys and simulated input are restricted.
- **Linux flyout:** it cannot be anchored to the tray icon.

## Testing

- **Committed tests:**
  - `transform`: line-break normalization, runs of breaks, CRLF, keep on/off, empty string.
  - `config`: parse valid input, and reject invalid input with a specific error.
- **Manual verification per platform** (not committed):
  - The hotkey pastes plain text.
  - Restore brings back RTF/HTML content.
  - The timeout path restores the snapshot.
- **Entry point:** a single Python script runs `cargo test` and enforces the global test budget.
