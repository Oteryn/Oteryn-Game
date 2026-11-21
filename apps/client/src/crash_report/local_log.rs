//! The in-memory log ring, the crash context and the local crash files.
use super::{redact, redaction::PLACEHOLDER};
use std::{
    collections::VecDeque,
    fs,
    io::{self, Write},
    panic::PanicHookInfo,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard, PoisonError, TryLockError},
    time::{SystemTime, UNIX_EPOCH},
};

/// Log lines kept in memory for a crash file.
pub const RECENT_LINES: usize = 200;
/// Directories under the client user-data directory.
pub const CRASH_DIRECTORY: &str = "crashes";
pub const LOG_DIRECTORY: &str = "logs";
const SEEN_MARKER: &str = "last-seen.txt";

/// The level class of one formatted log line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineLevel {
    Error,
    Warn,
    Other,
}

struct Recent {
    lines: VecDeque<String>,
    warnings: u64,
    errors: u64,
    last_error: Option<String>,
}
static RECENT: Mutex<Recent> = Mutex::new(Recent {
    lines: VecDeque::new(),
    warnings: 0,
    errors: 0,
    last_error: None,
});

/// What the client is doing, kept current by the host for a crash file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CrashContext {
    pub lifecycle: String,
    pub adapter: Option<String>,
}
static CONTEXT: Mutex<CrashContext> = Mutex::new(CrashContext {
    lifecycle: String::new(),
    adapter: None,
});

/// Warning and error counts since start, and the last error line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LogCounters {
    pub warnings: u64,
    pub errors: u64,
    pub last_error: Option<String>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A panic hook must not wait forever on a lock its own thread may hold.
fn try_lock<T>(mutex: &Mutex<T>) -> Option<MutexGuard<'_, T>> {
    for _ in 0..200 {
        match mutex.try_lock() {
            Ok(guard) => return Some(guard),
            Err(TryLockError::Poisoned(poisoned)) => return Some(poisoned.into_inner()),
            Err(TryLockError::WouldBlock) => std::thread::yield_now(),
        }
    }
    None
}

/// Keeps one already redacted, formatted log line and counts warnings and errors.
pub fn record_log_line(level: LineLevel, line: &str) {
    let line = line.trim_end();
    if line.is_empty() {
        return;
    }
    let mut recent = lock(&RECENT);
    if recent.lines.len() == RECENT_LINES {
        recent.lines.pop_front();
    }
    recent.lines.push_back(line.to_owned());
    match level {
        LineLevel::Error => {
            recent.errors = recent.errors.saturating_add(1);
            recent.last_error = Some(line.to_owned());
        }
        LineLevel::Warn => recent.warnings = recent.warnings.saturating_add(1),
        LineLevel::Other => {}
    }
}

/// The newest `limit` kept log lines, oldest first.
pub fn recent_log_lines(limit: usize) -> Vec<String> {
    let recent = lock(&RECENT);
    let skip = recent.lines.len().saturating_sub(limit);
    recent.lines.iter().skip(skip).cloned().collect()
}

pub fn log_counters() -> LogCounters {
    let recent = lock(&RECENT);
    LogCounters {
        warnings: recent.warnings,
        errors: recent.errors,
        last_error: recent.last_error.clone(),
    }
}

pub fn set_crash_context(context: CrashContext) {
    let mut held = lock(&CONTEXT);
    if *held != context {
        *held = context;
    }
}

/// The client user-data directory: the base of `startup.log`, `logs/` and `crashes/`.
pub fn user_data_directory() -> Option<PathBuf> {
    crate::native_login::settings_path()
        .ok()?
        .parent()
        .map(Path::to_path_buf)
}

/// A UTC calendar time from the system clock, without a time-zone database.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Utc {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub millisecond: u32,
}
impl Utc {
    pub fn at(time: SystemTime) -> Self {
        let since = time.duration_since(UNIX_EPOCH).unwrap_or_default();
        let seconds = since.as_secs();
        let days = i64::try_from(seconds / 86_400).unwrap_or(i64::MAX / 2);
        let of_day = seconds % 86_400;
        // Howard Hinnant's civil-from-days, for the proleptic Gregorian calendar.
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = doy - (153 * mp + 2) / 5 + 1;
        let month = if mp < 10 { mp + 3 } else { mp - 9 };
        let year = yoe + era * 400 + i64::from(month <= 2);
        let small = |value: u64| u32::try_from(value).unwrap_or(0);
        Self {
            year,
            month: u32::try_from(month).unwrap_or(1),
            day: u32::try_from(day).unwrap_or(1),
            hour: small(of_day / 3_600),
            minute: small(of_day % 3_600 / 60),
            second: small(of_day % 60),
            millisecond: since.subsec_millis(),
        }
    }
    /// `YYYY-MM-DD`.
    pub fn date(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
    /// `YYYY-MM-DDTHH-MM-SS.mmmZ`: sortable and valid in a Windows file name.
    pub fn file_stamp(self) -> String {
        format!(
            "{}T{:02}-{:02}-{:02}.{:03}Z",
            self.date(),
            self.hour,
            self.minute,
            self.second,
            self.millisecond
        )
    }
    /// RFC 3339 with milliseconds.
    pub fn rfc3339(self) -> String {
        format!(
            "{}T{:02}:{:02}:{:02}.{:03}Z",
            self.date(),
            self.hour,
            self.minute,
            self.second,
            self.millisecond
        )
    }
}

/// The text of one crash, before redaction.
pub struct Crash<'a> {
    pub message: &'a str,
    pub location: &'a str,
    pub thread: &'a str,
    pub backtrace: &'a str,
}

fn crash_text(crash: &Crash<'_>, now: SystemTime) -> String {
    let context = try_lock(&CONTEXT).map(|context| context.clone());
    let mut text = format!(
        "Oteryn client crash\nTime: {}\nBuild: {}\nOS: {} {}\nThread: {}\nLocation: {}\nMessage: {}\n",
        Utc::at(now).rfc3339(),
        crate::CLIENT_BUILD,
        std::env::consts::OS,
        std::env::consts::ARCH,
        crash.thread,
        crash.location,
        crash.message,
    );
    match &context {
        Some(context) => {
            let lifecycle = if context.lifecycle.is_empty() {
                "starting"
            } else {
                &context.lifecycle
            };
            text.push_str(&format!(
                "Lifecycle: {lifecycle}\nGraphics adapter: {}\n",
                context.adapter.as_deref().unwrap_or("not initialised"),
            ));
        }
        None => text.push_str("Lifecycle: unavailable (context locked)\n"),
    }
    text.push_str("\nBacktrace:\n");
    text.push_str(crash.backtrace);
    let mut text = redact(&text);
    if text == PLACEHOLDER {
        return text;
    }
    // Log events in the ring have already been checked as separate records.
    let lines = try_lock(&RECENT).map(|recent| recent.lines.iter().cloned().collect::<Vec<_>>());
    text.push_str("\n\nRecent log lines:\n");
    match lines {
        Some(lines) => {
            for line in lines {
                text.push_str(&line);
                text.push('\n');
            }
        }
        None => text.push_str("unavailable (log locked)\n"),
    }
    text
}

/// Writes `crash-<UTC timestamp>.log` in `directory` with owner-only permissions.
pub fn write_crash_file(
    directory: &Path,
    crash: &Crash<'_>,
    now: SystemTime,
) -> io::Result<PathBuf> {
    fs::create_dir_all(directory)?;
    let stamp = Utc::at(now).file_stamp();
    let text = crash_text(crash, now);
    let mut suffix = 0u32;
    loop {
        let name = if suffix == 0 {
            format!("crash-{stamp}.log")
        } else {
            format!("crash-{stamp}_{suffix}.log")
        };
        let path = directory.join(name);
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path) {
            Ok(mut file) => {
                file.write_all(text.as_bytes())?;
                file.sync_all()?;
                return Ok(path);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists && suffix < 100 => {
                suffix += 1;
            }
            Err(error) => return Err(error),
        }
    }
}

fn panic_message(info: &PanicHookInfo<'_>) -> String {
    let payload = info.payload();
    payload
        .downcast_ref::<&str>()
        .map(|text| (*text).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "non-text panic payload".to_owned())
}

/// Writes the crash file for one panic; the hook never panics itself.
pub fn record_panic(directory: &Path, info: &PanicHookInfo<'_>) -> io::Result<PathBuf> {
    let message = panic_message(info);
    let location = info
        .location()
        .map_or_else(|| "unknown".to_owned(), ToString::to_string);
    let thread = std::thread::current();
    let backtrace = std::backtrace::Backtrace::force_capture().to_string();
    write_crash_file(
        directory,
        &Crash {
            message: &message,
            location: &location,
            thread: thread.name().unwrap_or("unnamed"),
            backtrace: &backtrace,
        },
        SystemTime::now(),
    )
}

/// The newest crash file not reported before; marks it reported.
pub fn take_unseen_crash(directory: &Path) -> io::Result<Option<PathBuf>> {
    let newest = match fs::read_dir(directory) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| name.starts_with("crash-") && name.ends_with(".log"))
            .max(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let Some(newest) = newest else {
        return Ok(None);
    };
    let marker = directory.join(SEEN_MARKER);
    let seen = match fs::File::open(&marker) {
        Ok(file) => {
            use std::io::Read;
            const MAX_MARKER_BYTES: u64 = 1024;
            let mut seen = String::new();
            file.take(MAX_MARKER_BYTES + 1).read_to_string(&mut seen)?;
            if seen.len() as u64 > MAX_MARKER_BYTES {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "crash marker is too large"));
            }
            Some(seen)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    if seen.as_deref().is_some_and(|seen| seen.trim() >= newest.as_str()) {
        return Ok(None);
    }
    fs::write(&marker, format!("{newest}\n"))?;
    Ok(Some(directory.join(newest)))
}

#[cfg(test)]
mod redaction_tests;
#[cfg(test)]
mod tests;
