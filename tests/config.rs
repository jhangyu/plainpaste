use std::path::Path;

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use plainpaste::config::{ConfigError, Settings, Theme, parse_settings, serialize_settings};

const PATH: &str = "/cfg/settings.json";

fn parse(text: &str) -> Result<Settings, ConfigError> {
    parse_settings(text, Path::new(PATH))
}

fn expect_invalid(text: &str, want_field: &str, want_actual_contains: &str) {
    match parse(text) {
        Err(ConfigError::Invalid {
            path,
            field,
            expected,
            actual,
        }) => {
            assert_eq!(path, Path::new(PATH));
            assert_eq!(field, want_field);
            assert!(!expected.is_empty());
            assert!(
                actual.contains(want_actual_contains),
                "actual `{actual}` should contain `{want_actual_contains}`"
            );
        }
        other => panic!("expected ConfigError::Invalid for field {want_field}, got {other:?}"),
    }
}

#[test]
fn parses_valid_document() {
    let text = r#"{"hotkey":"control+shift+KeyV","keep_line_breaks":true,"restore_clipboard":false,"theme":"dark"}"#;
    let settings = parse(text).expect("valid document");
    assert_eq!(
        settings,
        Settings {
            hotkey: HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyV),
            keep_line_breaks: true,
            restore_clipboard: false,
            theme: Theme::Dark,
        }
    );
}

#[test]
fn serialize_then_parse_round_trips_defaults() {
    let settings = Settings::default();
    assert_eq!(
        parse(&serialize_settings(&settings)).expect("round trip"),
        settings
    );
}

#[test]
fn rejects_malformed_json() {
    expect_invalid("{", "<json>", "EOF");
}

#[test]
fn rejects_non_object_root() {
    expect_invalid("[]", "<root>", "[]");
}

#[test]
fn rejects_wrong_bool_type() {
    expect_invalid(
        r#"{"hotkey":"control+shift+KeyV","keep_line_breaks":"yes","restore_clipboard":true,"theme":"system"}"#,
        "keep_line_breaks",
        "\"yes\"",
    );
}

#[test]
fn rejects_missing_field() {
    expect_invalid(
        r#"{"hotkey":"control+shift+KeyV","keep_line_breaks":false,"theme":"system"}"#,
        "restore_clipboard",
        "missing",
    );
}

#[test]
fn rejects_unknown_theme() {
    expect_invalid(
        r#"{"hotkey":"control+shift+KeyV","keep_line_breaks":false,"restore_clipboard":true,"theme":"blue"}"#,
        "theme",
        "\"blue\"",
    );
}

#[test]
fn rejects_unknown_field() {
    expect_invalid(
        r#"{"hotkey":"control+shift+KeyV","keep_line_breaks":false,"restore_clipboard":true,"theme":"system","extra":1}"#,
        "extra",
        "unknown",
    );
}

#[test]
fn rejects_unparsable_hotkey() {
    expect_invalid(
        r#"{"hotkey":"shift+KeyV+alt","keep_line_breaks":false,"restore_clipboard":true,"theme":"system"}"#,
        "hotkey",
        "shift+KeyV+alt",
    );
}

#[test]
fn rejects_hotkey_without_modifier() {
    expect_invalid(
        r#"{"hotkey":"KeyV","keep_line_breaks":false,"restore_clipboard":true,"theme":"system"}"#,
        "hotkey",
        "no modifier",
    );
}
