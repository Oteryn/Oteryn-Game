//! Client log output: one redacted formatter feeding stderr, a rolling daily file and the
//! in-memory ring a crash file reads.
use crate::crash_report::{LineLevel, Utc, record_log_line, redact};
#[cfg(test)]
use bevy::log::tracing_subscriber::layer::SubscriberExt;
use bevy::{
    app::App,
    log::{
        BoxedFmtLayer, BoxedLayer, Level, LogPlugin,
        tracing::Metadata,
        tracing_subscriber::{EnvFilter, Layer, Registry, fmt, reload},
    },
};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock, PoisonError, TryLockError, mpsc},
    thread,
    time::SystemTime,
};

/// Daily files kept, the current one included.
pub(super) const KEEP_FILES: usize = 7;
/// The total size of the log directory.
pub(super) const TOTAL_CAP: u64 = 50 * 1024 * 1024;
/// A full filter for the whole process; it replaces the Options level while set.
pub(super) const ENV_OVERRIDE: &str = "OTERYN_LOG";

/// `logs/client-YYYY-MM-DD.log`, at most `keep` files and `cap` bytes together.
pub(super) struct RollingFile {
    directory: PathBuf,
    keep: usize,
    cap: u64,
    capped_date: Mutex<Option<String>>,
}
const CAPPED: &str = "log size cap reached; further lines today go to the crash ring only\n";

impl RollingFile {
    pub fn new(directory: PathBuf, keep: usize, cap: u64) -> Self {
        Self {
            directory,
            keep: keep.max(1),
            cap,
            capped_date: Mutex::default(),
        }
    }
    pub fn directory(&self) -> &Path {
        &self.directory
    }
    pub fn write_text(&self, now: SystemTime, text: &str) -> io::Result<()> {
        self.write_dated(&Utc::at(now).date(), text)
    }
    pub(super) fn write_dated(&self, date: &str, text: &str) -> io::Result<()> {
        let mut capped_date = match self.capped_date.try_lock() {
            Ok(state) => state,
            Err(TryLockError::Poisoned(error)) => error.into_inner(),
            Err(TryLockError::WouldBlock) => return Err(io::ErrorKind::WouldBlock.into()),
        };
        if capped_date.as_deref() == Some(date) {
            return Ok(());
        }
        *capped_date = None;
        fs::create_dir_all(&self.directory)?;
        let lock = fs::File::options()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.directory.join(".lock"))?;
        lock.try_lock().map_err(io::Error::from)?;
        let name = format!("client-{date}.log");
        let mut file = fs::File::options()
            .create(true)
            .append(true)
            .open(self.directory.join(&name))?;
        let length = text.len() as u64;
        let size = file.metadata()?.len();
        let budget = self.cap.saturating_sub(self.prune(&name)?);
        if size + length + CAPPED.len() as u64 > budget {
            if size + CAPPED.len() as u64 <= budget {
                file.write_all(CAPPED.as_bytes())?;
            }
            *capped_date = Some(date.to_owned());
            return Ok(());
        }
        file.write_all(text.as_bytes())
    }
    /// Removes the oldest other files beyond the count, then until the others use at most half
    /// of the cap; returns the size the others keep.
    fn prune(&self, current: &str) -> io::Result<u64> {
        let mut others = BTreeMap::new();
        for entry in fs::read_dir(&self.directory)? {
            let entry = entry?;
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            if name != current && name.starts_with("client-") && name.ends_with(".log") {
                others.insert(name, entry.metadata()?.len());
                if others.len() >= self.keep && let Some((oldest, _)) = others.pop_first() {
                    fs::remove_file(self.directory.join(oldest))?;
                }
            }
        }
        let mut total = others.values().copied().fold(0_u64, u64::saturating_add);
        for (name, size) in others {
            if total <= self.cap / 2 {
                break;
            }
            fs::remove_file(self.directory.join(name))?;
            total -= size;
        }
        Ok(total)
    }
}

type LogWrite = (SystemTime, String);

fn file_writer(file: RollingFile) -> io::Result<(mpsc::SyncSender<LogWrite>, thread::JoinHandle<()>)> {
    let (sender, receiver) = mpsc::sync_channel::<LogWrite>(256);
    let worker = thread::Builder::new().name("client-log".into()).spawn(move || {
        for (at, text) in receiver {
            let _ = file.write_text(at, &text);
        }
    })?;
    Ok((sender, worker))
}

/// One formatted event is redacted once, then written to every sink.
#[derive(Clone)]
pub(super) struct Sink {
    file: Option<mpsc::SyncSender<LogWrite>>,
    #[cfg(test)]
    synchronous_file: Option<std::sync::Arc<RollingFile>>,
    stderr: bool,
    ring: bool,
}
pub(super) struct SinkWriter {
    sink: Sink,
    level: LineLevel,
    buffer: Vec<u8>,
}
impl<'a> fmt::MakeWriter<'a> for Sink {
    type Writer = SinkWriter;
    fn make_writer(&'a self) -> SinkWriter {
        SinkWriter {
            sink: self.clone(),
            level: LineLevel::Other,
            buffer: Vec::new(),
        }
    }
    fn make_writer_for(&'a self, meta: &Metadata<'_>) -> SinkWriter {
        let level = match *meta.level() {
            Level::ERROR => LineLevel::Error,
            Level::WARN => LineLevel::Warn,
            _ => LineLevel::Other,
        };
        SinkWriter {
            sink: self.clone(),
            level,
            buffer: Vec::new(),
        }
    }
}
impl Write for SinkWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Drop for SinkWriter {
    fn drop(&mut self) {
        if self.buffer.is_empty() {
            return;
        }
        let record = String::from_utf8_lossy(&self.buffer);
        let text = if let Some(record) = record.strip_suffix('\n') {
            let mut text = redact(record);
            text.push('\n');
            text
        } else {
            redact(&record)
        };
        if self.sink.stderr {
            let _ = io::stderr().write_all(text.as_bytes());
        }
        if self.sink.ring {
            let mut level = self.level;
            for line in text.lines() {
                record_log_line(level, line);
                level = LineLevel::Other;
            }
        }
        #[cfg(test)]
        if let Some(file) = &self.sink.synchronous_file {
            let _ = file.write_text(SystemTime::now(), &text);
        }
        if let Some(file) = &self.sink.file {
            // On saturation the line remains available in stderr and the crash ring.
            let _ = file.try_send((SystemTime::now(), text));
        }
    }
}

/// Third-party targets that can carry request, token or handshake detail. They stay at `warn`
/// whatever the level, unless `OTERYN_LOG` names them; redaction is only the backstop.
const CAPPED_TARGETS: [&str; 15] = [
    "h2",
    "hyper",
    "hyper_util",
    "naga",
    "oauth2",
    "reqwest",
    "rustls",
    "tokio_rustls",
    "tokio_tungstenite",
    "tungstenite",
    "ureq",
    "webpki",
    "wgpu",
    "wgpu_core",
    "wgpu_hal",
];

/// The filter for an Options level (`warn`, `info` or `debug`), or for an `OTERYN_LOG` value,
/// with every capped target not named in it held at `warn`.
pub(super) fn level_filter(directives: &str) -> EnvFilter {
    let mut text = directives.trim().to_owned();
    for target in CAPPED_TARGETS {
        let named = directives.split(',').any(|directive| {
            directive
                .trim()
                .split(['=', '['])
                .next()
                .is_some_and(|name| name == target || name.starts_with(&format!("{target}::")))
        });
        if !named {
            text.push_str(&format!(",{target}=warn"));
        }
    }
    EnvFilter::builder().parse_lossy(text)
}

/// The formatter every sink shares: UTC timestamp, level, target, thread name and id.
pub(super) fn format_layer<S>(sink: Sink) -> impl bevy::log::tracing_subscriber::Layer<S>
where
    S: bevy::log::tracing::Subscriber
        + for<'span> bevy::log::tracing_subscriber::registry::LookupSpan<'span>,
{
    fmt::layer()
        .with_ansi(false)
        .with_target(true)
        .with_thread_names(true)
        .with_thread_ids(true)
        .with_writer(sink)
}

/// The production filter and file formatter, for tests that read what was logged.
#[cfg(test)]
pub(super) fn test_subscriber(
    file: &std::sync::Arc<RollingFile>,
    directives: &str,
) -> impl bevy::log::tracing::Subscriber {
    Registry::default()
        .with(level_filter(directives))
        .with(format_layer(Sink {
            file: None,
            synchronous_file: Some(std::sync::Arc::clone(file)),
            stderr: false,
            ring: true,
        }))
}

/// What `prepare` hands the `LogPlugin` layer hooks, which are plain function pointers.
struct Setup {
    base: Option<PathBuf>,
    level: &'static str,
}
static SETUP: Mutex<Option<Setup>> = Mutex::new(None);

struct Control {
    handle: reload::Handle<EnvFilter, Registry>,
    directory: Option<PathBuf>,
}
static CONTROL: OnceLock<Control> = OnceLock::new();

fn environment() -> Option<String> {
    std::env::var(ENV_OVERRIDE)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

/// Bevy's `LogPlugin` with the Oteryn layers: a reloadable filter, the rolling file and crash
/// ring (custom layer) and a redacted stderr formatter (replacing Bevy's unredacted one). The
/// Options level comes from the saved preferences, read before the plugin starts.
pub(in crate::bevy_shell) fn plugin(base: Option<PathBuf>, level: &'static str) -> LogPlugin {
    *SETUP.lock().unwrap_or_else(PoisonError::into_inner) = Some(Setup { base, level });
    let environment = environment();
    LogPlugin {
        // Bevy's own gate stays wide; the reloadable Oteryn filter decides.
        level: if environment.is_some() {
            Level::TRACE
        } else {
            Level::DEBUG
        },
        filter: environment.map_or_else(
            || bevy::log::DEFAULT_FILTER.to_owned(),
            |value| format!("{},{value}", bevy::log::DEFAULT_FILTER),
        ),
        custom_layer,
        fmt_layer,
    }
}

fn custom_layer(_: &mut App) -> Option<BoxedLayer> {
    let setup = SETUP
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take()?;
    let directives = environment().unwrap_or_else(|| setup.level.to_owned());
    let (filter, handle) = reload::Layer::new(level_filter(&directives));
    let file = setup.base.map(|base| {
        RollingFile::new(
            base.join(crate::crash_report::LOG_DIRECTORY),
            KEEP_FILES,
            TOTAL_CAP,
        )
    });
    let directory = file.as_ref().map(|file| file.directory().to_path_buf());
    let _ = CONTROL.set(Control { handle, directory });
    let sink = Sink {
        file: file.and_then(|file| file_writer(file).ok().map(|(sender, _)| sender)),
        #[cfg(test)]
        synchronous_file: None,
        stderr: false,
        ring: true,
    };
    Some(Box::new(filter.and_then(format_layer(sink))))
}

fn fmt_layer(_: &mut App) -> Option<BoxedFmtLayer> {
    Some(Box::new(format_layer(Sink {
        file: None,
        #[cfg(test)]
        synchronous_file: None,
        stderr: true,
        ring: false,
    })))
}

/// Where the active filter came from, for the startup line.
pub(in crate::bevy_shell) fn filter_source() -> &'static str {
    if environment().is_some() {
        ENV_OVERRIDE
    } else {
        "Options"
    }
}

/// Applies an Options level at once, unless `OTERYN_LOG` set the filter.
pub(in crate::bevy_shell) fn set_level(level: &str) {
    if let Some(control) = CONTROL.get()
        && environment().is_none()
    {
        let _ = control.handle.reload(level_filter(level));
    }
}

/// The directory of the rolling log files, once logging is installed.
pub(in crate::bevy_shell) fn directory() -> Option<PathBuf> {
    CONTROL.get().and_then(|control| control.directory.clone())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "../../crash_report/rolling_log_tests.rs"]
mod rolling_log_tests;
