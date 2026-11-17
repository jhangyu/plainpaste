//! Settings: parsed once at the boundary into typed values, saved atomically.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use serde_json::{Map, Value};

use crate::hotkey::PRIMARY_MODIFIER;

const APP_DIR_NAME: &str = "PlainPaste";
const CONFIG_FILE_NAME: &str = "settings.json";
const TEMP_SUFFIX: &str = "tmp";
#[cfg(any(windows, all(unix, not(target_os = "macos"))))]
const LOGS_SUBDIR: &str = "logs";

const FIELD_HOTKEY: &str = "hotkey";
const FIELD_KEEP_LINE_BREAKS: &str = "keep_line_breaks";
const FIELD_RESTORE_CLIPBOARD: &str = "restore_clipboard";
const FIELD_THEME: &str = "theme";
const KNOWN_FIELDS: [&str; 4] = [
    FIELD_HOTKEY,
    FIELD_KEEP_LINE_BREAKS,
    FIELD_RESTORE_CLIPBOARD,
    FIELD_THEME,
];
const ROOT_FIELD: &str = "<root>";
const JSON_FIELD: &str = "<json>";

const THEME_SYSTEM: &str = "system";
const THEME_LIGHT: &str = "light";
const THEME_DARK: &str = "dark";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    System,
    Light,
    Dark,
}

impl Theme {
    pub fn as_str(self) -> &'static str {
        match self {
            Theme::System => THEME_SYSTEM,
            Theme::Light => THEME_LIGHT,
            Theme::Dark => THEME_DARK,
        }
    }

    pub fn parse(value: &str) -> Option<Theme> {
        match value {
            THEME_SYSTEM => Some(Theme::System),
            THEME_LIGHT => Some(Theme::Light),
            THEME_DARK => Some(Theme::Dark),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    pub hotkey: HotKey,
    pub keep_line_breaks: bool,
    pub restore_clipboard: bool,
    pub theme: Theme,
}

/// Cmd+Shift+V on macOS, Ctrl+Shift+V elsewhere (spec: Settings).
pub fn default_hotkey() -> HotKey {
    HotKey::new(Some(PRIMARY_MODIFIER | Modifiers::SHIFT), Code::KeyV)
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            hotkey: default_hotkey(),
            keep_line_breaks: false,
            restore_clipboard: true,
            theme: Theme::System,
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    /// The file content does not match the schema.
    Invalid {
        path: PathBuf,
        field: String,
        expected: String,
        actual: String,
    },
    /// A filesystem operation on the config file failed.
    Io {
        path: PathBuf,
        op: &'static str,
        source: io::Error,
    },
    /// The OS reports no home or config directory.
    NoConfigDir,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Invalid {
                path,
                field,
                expected,
                actual,
            } => write!(
                f,
                "config file {}: field `{field}`: expected {expected}, actual {actual}",
                path.display()
            ),
            ConfigError::Io { path, op, source } => {
                write!(f, "config file {}: {op} failed: {source}", path.display())
            }
            ConfigError::NoConfigDir => write!(
                f,
                "config directory: expected a platform config directory, actual none (no home directory)"
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

fn invalid(path: &Path, field: &str, expected: &str, actual: String) -> ConfigError {
    ConfigError::Invalid {
        path: path.to_path_buf(),
        field: field.to_string(),
        expected: expected.to_string(),
        actual,
    }
}

fn require<'a>(
    obj: &'a Map<String, Value>,
    path: &Path,
    field: &str,
    expected: &str,
) -> Result<&'a Value, ConfigError> {
    obj.get(field)
        .ok_or_else(|| invalid(path, field, expected, "missing".to_string()))
}

fn parse_bool(obj: &Map<String, Value>, path: &Path, field: &str) -> Result<bool, ConfigError> {
    const EXPECTED: &str = "true or false";
    let value = require(obj, path, field, EXPECTED)?;
    value
        .as_bool()
        .ok_or_else(|| invalid(path, field, EXPECTED, value.to_string()))
}

fn parse_hotkey(obj: &Map<String, Value>, path: &Path) -> Result<HotKey, ConfigError> {
    const EXPECTED: &str = "a hotkey string with at least one modifier, like \"shift+meta+KeyV\"";
    let value = require(obj, path, FIELD_HOTKEY, EXPECTED)?;
    let text = value
        .as_str()
        .ok_or_else(|| invalid(path, FIELD_HOTKEY, EXPECTED, value.to_string()))?;
    let hotkey: HotKey = text
        .parse()
        .map_err(|e| invalid(path, FIELD_HOTKEY, EXPECTED, format!("{value} ({e})")))?;
    if hotkey.mods.is_empty() {
        return Err(invalid(
            path,
            FIELD_HOTKEY,
            EXPECTED,
            format!("{value} (no modifier)"),
        ));
    }
    Ok(hotkey)
}

fn parse_theme(obj: &Map<String, Value>, path: &Path) -> Result<Theme, ConfigError> {
    const EXPECTED: &str = "one of \"system\", \"light\", \"dark\"";
    let value = require(obj, path, FIELD_THEME, EXPECTED)?;
    value
        .as_str()
        .and_then(Theme::parse)
        .ok_or_else(|| invalid(path, FIELD_THEME, EXPECTED, value.to_string()))
}

/// Parses the JSON settings document. `path` is used only in error messages.
pub fn parse_settings(text: &str, path: &Path) -> Result<Settings, ConfigError> {
    let root: Value = serde_json::from_str(text)
        .map_err(|e| invalid(path, JSON_FIELD, "a valid JSON document", e.to_string()))?;
    let obj = root
        .as_object()
        .ok_or_else(|| invalid(path, ROOT_FIELD, "a JSON object", root.to_string()))?;
    if let Some(unknown) = obj.keys().find(|k| !KNOWN_FIELDS.contains(&k.as_str())) {
        return Err(invalid(
            path,
            unknown,
            &format!("one of {}", KNOWN_FIELDS.join(", ")),
            "an unknown field".to_string(),
        ));
    }
    Ok(Settings {
        hotkey: parse_hotkey(obj, path)?,
        keep_line_breaks: parse_bool(obj, path, FIELD_KEEP_LINE_BREAKS)?,
        restore_clipboard: parse_bool(obj, path, FIELD_RESTORE_CLIPBOARD)?,
        theme: parse_theme(obj, path)?,
    })
}

pub fn serialize_settings(settings: &Settings) -> String {
    let doc = serde_json::json!({
        FIELD_HOTKEY: settings.hotkey.into_string(),
        FIELD_KEEP_LINE_BREAKS: settings.keep_line_breaks,
        FIELD_RESTORE_CLIPBOARD: settings.restore_clipboard,
        FIELD_THEME: settings.theme.as_str(),
    });
    serde_json::to_string_pretty(&doc).expect("a serde_json::Value always serializes")
}

/// Home directory: `$HOME` if non-empty, else the password database.
#[cfg(unix)]
fn home_dir() -> Option<PathBuf> {
    // ponytail: std's unix home_dir is exactly this rule (its deprecation concerns Windows only).
    #[allow(deprecated)]
    std::env::home_dir()
}

/// An XDG base directory: the variable if it is an absolute path, else `$HOME/<fallback>`.
#[cfg(all(unix, not(target_os = "macos")))]
fn xdg_dir(var: &str, fallback: &str) -> Option<PathBuf> {
    let absolute = std::env::var_os(var)
        .map(PathBuf::from)
        .filter(|p| p.is_absolute());
    absolute.or_else(|| home_dir().map(|home| home.join(fallback)))
}

/// Lowercased app name without whitespace, the XDG folder name.
#[cfg(all(unix, not(target_os = "macos")))]
fn xdg_app_name() -> String {
    APP_DIR_NAME
        .split_whitespace()
        .map(str::to_lowercase)
        .collect()
}

/// A Windows Known Folder, so redirected folders resolve as the OS resolves them.
#[cfg(windows)]
fn known_folder(id: &windows::core::GUID) -> Option<PathBuf> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{KNOWN_FOLDER_FLAG, SHGetKnownFolderPath};
    // SAFETY: on success the call returns a NUL-terminated string that is copied, then freed once.
    unsafe {
        let ptr = SHGetKnownFolderPath(id, KNOWN_FOLDER_FLAG(0), None).ok()?;
        let path = PathBuf::from(OsString::from_wide(ptr.as_wide()));
        CoTaskMemFree(Some(ptr.0 as *const _));
        Some(path)
    }
}

/// Roaming and local app-data folders. Both must resolve.
#[cfg(windows)]
fn app_data_dirs() -> Option<(PathBuf, PathBuf)> {
    use windows::Win32::UI::Shell::{FOLDERID_LocalAppData, FOLDERID_RoamingAppData};
    Some((
        known_folder(&FOLDERID_RoamingAppData)?,
        known_folder(&FOLDERID_LocalAppData)?,
    ))
}

/// macOS: `~/Library/Application Support/PlainPaste`.
#[cfg(target_os = "macos")]
pub(crate) fn config_dir() -> Option<PathBuf> {
    let name = APP_DIR_NAME.replace(' ', "-");
    Some(home_dir()?.join("Library/Application Support").join(name))
}

/// macOS: `~/Library/Logs/PlainPaste` (the standard log folder).
#[cfg(target_os = "macos")]
pub(crate) fn log_dir() -> Option<PathBuf> {
    let name = APP_DIR_NAME.replace(' ', "-");
    Some(home_dir()?.join("Library/Logs").join(name))
}

/// Other Unix: `$XDG_CONFIG_HOME/plainpaste`.
#[cfg(all(unix, not(target_os = "macos")))]
pub(crate) fn config_dir() -> Option<PathBuf> {
    home_dir()?;
    Some(xdg_dir("XDG_CONFIG_HOME", ".config")?.join(xdg_app_name()))
}

/// Other Unix: `$XDG_DATA_HOME/plainpaste/logs`.
#[cfg(all(unix, not(target_os = "macos")))]
pub(crate) fn log_dir() -> Option<PathBuf> {
    home_dir()?;
    Some(
        xdg_dir("XDG_DATA_HOME", ".local/share")?
            .join(xdg_app_name())
            .join(LOGS_SUBDIR),
    )
}

/// Windows: `<RoamingAppData>\PlainPaste\config`.
#[cfg(windows)]
pub(crate) fn config_dir() -> Option<PathBuf> {
    Some(app_data_dirs()?.0.join(APP_DIR_NAME).join("config"))
}

/// Windows: `<LocalAppData>\PlainPaste\data\logs`.
#[cfg(windows)]
pub(crate) fn log_dir() -> Option<PathBuf> {
    Some(
        app_data_dirs()?
            .1
            .join(APP_DIR_NAME)
            .join("data")
            .join(LOGS_SUBDIR),
    )
}

/// `<platform config dir>/settings.json`.
pub fn settings_path() -> Result<PathBuf, ConfigError> {
    config_dir()
        .map(|dir| dir.join(CONFIG_FILE_NAME))
        .ok_or(ConfigError::NoConfigDir)
}

/// Loads the settings file. A missing file is created with the defaults.
/// A file that fails to parse is an error and is never overwritten.
pub fn load_or_create(path: &Path) -> Result<Settings, ConfigError> {
    match fs::read_to_string(path) {
        Ok(text) => parse_settings(&text, path),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let settings = Settings::default();
            save(path, &settings)?;
            log::info!("created config file {} with defaults", path.display());
            Ok(settings)
        }
        Err(source) => Err(ConfigError::Io {
            path: path.to_path_buf(),
            op: "read",
            source,
        }),
    }
}

/// Writes to a sibling temp file, then renames it over `path`.
pub fn save(path: &Path, settings: &Settings) -> Result<(), ConfigError> {
    let io_err = |op: &'static str| {
        move |source| ConfigError::Io {
            path: path.to_path_buf(),
            op,
            source,
        }
    };
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(io_err("create directory"))?;
    }
    let temp = path.with_extension(TEMP_SUFFIX);
    fs::write(&temp, serialize_settings(settings)).map_err(io_err("write temp file"))?;
    fs::rename(&temp, path).map_err(io_err("rename temp file"))
}
