//! Log file: location, sinks and logger setup.

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use log::{LevelFilter, Log, Metadata, Record, SetLoggerError};

use crate::config::log_dir;

const LOG_FILE_NAME: &str = "plainpaste.log";
const DEFAULT_LOG_FILTER: &str = "info";
const LOG_FILTER_ENV: &str = "RUST_LOG";
const SECONDS_PER_DAY: u64 = 86_400;
const SECONDS_PER_HOUR: u64 = 3_600;
const SECONDS_PER_MINUTE: u64 = 60;

#[derive(Debug)]
pub enum LogError {
    /// The OS reports no home or data directory for the log file.
    NoLogDir,
    /// The log file cannot be created or appended to.
    Open { path: PathBuf, source: io::Error },
    /// A global logger is already installed.
    Install { source: SetLoggerError },
}

impl fmt::Display for LogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogError::NoLogDir => write!(
                f,
                "log directory: expected a platform home or data directory, actual none (no home directory)"
            ),
            LogError::Open { path, source } => write!(
                f,
                "log file {}: expected create/append, actual error: {source}",
                path.display()
            ),
            LogError::Install { source } => write!(
                f,
                "global logger: expected none installed, actual error: {source}"
            ),
        }
    }
}

impl std::error::Error for LogError {}

/// macOS: `~/Library/Logs/<app>/plainpaste.log` (the standard log folder).
/// Elsewhere: `<platform local data dir>/<app>/logs/plainpaste.log`.
pub fn log_path() -> Result<PathBuf, LogError> {
    Ok(log_dir().ok_or(LogError::NoLogDir)?.join(LOG_FILE_NAME))
}

/// Writes every log record to the log file and to stderr. Launched from Finder,
/// stderr goes nowhere, so the file is the record. From a terminal, both work.
struct LogSinks {
    file: File,
}

impl Write for LogSinks {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write_all(buf)?;
        io::stderr().write_all(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()?;
        io::stderr().flush()
    }
}

// ponytail: the log file is appended to and never rotated. Add rotation if it grows too large in practice.
fn open_log_file(path: &Path) -> io::Result<File> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    OpenOptions::new().create(true).append(true).open(path)
}

/// Level filter from `RUST_LOG` syntax: `level`, `target=level`, or a bare `target` (trace).
struct Filter {
    /// Sorted by target length, the longest prefix matches first.
    directives: Vec<(Option<String>, LevelFilter)>,
}

impl Filter {
    fn parse(spec: &str) -> Filter {
        let mut directives = Vec::new();
        for part in spec.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            match Self::parse_directive(part) {
                Some(directive) => directives.push(directive),
                None => eprintln!(
                    "{LOG_FILTER_ENV}: expected `level`, `target` or `target=level`, actual `{part}` (skipped)"
                ),
            }
        }
        if directives.is_empty() {
            directives.push((None, LevelFilter::Error));
        }
        directives.sort_by_key(|(target, _)| target.as_ref().map_or(0, String::len));
        Filter { directives }
    }

    fn parse_directive(part: &str) -> Option<(Option<String>, LevelFilter)> {
        match part.split_once('=') {
            None => match LevelFilter::from_str(part) {
                Ok(level) => Some((None, level)),
                Err(_) => Some((Some(part.to_string()), LevelFilter::Trace)),
            },
            Some((target, level)) => {
                let level = LevelFilter::from_str(level.trim()).ok()?;
                Some((Some(target.trim().to_string()), level))
            }
        }
    }

    fn max_level(&self) -> LevelFilter {
        self.directives
            .iter()
            .map(|(_, level)| *level)
            .max()
            .unwrap_or(LevelFilter::Off)
    }

    fn enabled(&self, metadata: &Metadata) -> bool {
        let target = metadata.target();
        self.directives
            .iter()
            .rev()
            .find(|(name, _)| name.as_ref().is_none_or(|n| target.starts_with(n.as_str())))
            .is_some_and(|(_, level)| metadata.level() <= *level)
    }
}

struct FileLogger {
    filter: Filter,
    sinks: Mutex<LogSinks>,
}

/// Days since 1970-01-01 to (year, month, day) in the proleptic Gregorian calendar.
fn civil_from_days(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let day_of_era = z % 146_097;
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 { month_index + 3 } else { month_index - 9 };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    (year, month, day)
}

fn format_line(epoch_seconds: u64, level: log::Level, target: &str, args: &fmt::Arguments) -> String {
    let (year, month, day) = civil_from_days(epoch_seconds / SECONDS_PER_DAY);
    let in_day = epoch_seconds % SECONDS_PER_DAY;
    let hour = in_day / SECONDS_PER_HOUR;
    let minute = in_day % SECONDS_PER_HOUR / SECONDS_PER_MINUTE;
    let second = in_day % SECONDS_PER_MINUTE;
    format!(
        "[{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z {level:<5} {target}] {args}\n"
    )
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.filter.enabled(metadata)
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let line = format_line(now, record.level(), record.target(), record.args());
        let mut sinks = self.sinks.lock().unwrap_or_else(|e| e.into_inner());
        // A failed log write has nowhere left to be reported.
        let _ = sinks.write(line.as_bytes());
    }

    fn flush(&self) {
        let mut sinks = self.sinks.lock().unwrap_or_else(|e| e.into_inner());
        let _ = sinks.flush();
    }
}

/// Installs the global logger and returns the log file path.
pub fn init() -> Result<PathBuf, LogError> {
    let path = log_path()?;
    let file = open_log_file(&path).map_err(|source| LogError::Open {
        path: path.clone(),
        source,
    })?;
    let spec = std::env::var(LOG_FILTER_ENV).unwrap_or_else(|_| DEFAULT_LOG_FILTER.to_string());
    let filter = Filter::parse(&spec);
    let max_level = filter.max_level();
    let logger = FileLogger {
        filter,
        sinks: Mutex::new(LogSinks { file }),
    };
    log::set_boxed_logger(Box::new(logger)).map_err(|source| LogError::Install { source })?;
    log::set_max_level(max_level);
    log::info!("logging to {}", path.display());
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_format_is_stable() {
        let line = format_line(1_709_251_198, log::Level::Info, "a::b", &format_args!("hi"));
        assert_eq!(line, "[2024-02-29T23:59:58Z INFO  a::b] hi\n");
    }

    #[test]
    fn filter_uses_longest_prefix() {
        let filter = Filter::parse("warn,a=debug,a::b=error");
        let at = |target: &str, level| {
            filter.enabled(&Metadata::builder().target(target).level(level).build())
        };
        assert!(at("a::c", log::Level::Debug));
        assert!(!at("a::b", log::Level::Warn));
        assert!(at("z", log::Level::Warn));
        assert!(!at("z", log::Level::Info));
        assert_eq!(filter.max_level(), LevelFilter::Debug);
    }
}
