//! Plain-text transform applied to clipboard text before it is pasted.

const LINE_FEED: char = '\n';
const CARRIAGE_RETURN: char = '\r';
pub const LINE_BREAK_REPLACEMENT: char = ' ';

/// Normalizes every line break (`\r\n`, `\r`, `\n`) to `\n`.
/// When `keep_line_breaks` is false, each run of consecutive line breaks becomes one space.
pub fn normalize_line_breaks(text: &str, keep_line_breaks: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_break_run = false;
    while let Some(c) = chars.next() {
        let is_break = c == LINE_FEED || c == CARRIAGE_RETURN;
        if !is_break {
            in_break_run = false;
            out.push(c);
            continue;
        }
        if c == CARRIAGE_RETURN && chars.peek() == Some(&LINE_FEED) {
            chars.next();
        }
        if keep_line_breaks {
            out.push(LINE_FEED);
        } else if !in_break_run {
            out.push(LINE_BREAK_REPLACEMENT);
        }
        in_break_run = true;
    }
    out
}
