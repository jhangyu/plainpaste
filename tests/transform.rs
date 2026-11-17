use plainpaste::transform::normalize_line_breaks;

#[test]
fn empty_string_stays_empty() {
    assert_eq!(normalize_line_breaks("", false), "");
    assert_eq!(normalize_line_breaks("", true), "");
}

#[test]
fn text_without_breaks_is_unchanged() {
    assert_eq!(normalize_line_breaks("a b\tc", false), "a b\tc");
    assert_eq!(normalize_line_breaks("a b\tc", true), "a b\tc");
}

#[test]
fn keep_on_normalizes_crlf_and_cr_to_lf() {
    assert_eq!(normalize_line_breaks("a\r\nb\rc\nd", true), "a\nb\nc\nd");
}

#[test]
fn keep_on_preserves_every_break_in_a_run() {
    assert_eq!(normalize_line_breaks("a\r\n\r\nb", true), "a\n\nb");
}

#[test]
fn keep_off_replaces_single_breaks_with_space() {
    assert_eq!(
        normalize_line_breaks("一行\n二行\r\n三行\r四行", false),
        "一行 二行 三行 四行"
    );
}

#[test]
fn keep_off_collapses_mixed_runs_to_one_space() {
    assert_eq!(normalize_line_breaks("a\r\n\n\r\r\nb", false), "a b");
}

#[test]
fn keep_off_keeps_leading_and_trailing_runs_as_one_space() {
    assert_eq!(normalize_line_breaks("\n\na\r\n", false), " a ");
}

#[test]
fn crlf_counts_as_one_break_not_two() {
    assert_eq!(normalize_line_breaks("a\r\nb", true), "a\nb");
}
