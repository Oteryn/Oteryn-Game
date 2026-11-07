//! The §1.10 item 1 panic hook: one `E4001 PANIC` line, no default hook, no backtrace.

use crate::line::{Level, Line, unix_ms};

crate::error_kinds! {
    /// Runtime-internal codes (4xxx) owned by this crate.
    pub enum PanicKind {
        Panic = (4001, "PANIC", InternalUnavailable, Terminal),
    }
}

/// `E4001 PANIC`.
pub const PANIC: crate::ErrorCode = PanicKind::Panic.code();

/// The panic line. The payload goes into `detail` only when it is a `&'static str`; any other
/// payload may carry runtime values, so it is withheld as `payload=formatted`.
#[must_use]
pub fn panic_line(
    process: &str,
    build: &str,
    trace: Option<&str>,
    thread: &str,
    location: Option<(&str, u32, u32)>,
    payload: Option<&'static str>,
) -> String {
    let at = location.map_or_else(
        || "unknown".to_owned(),
        |(file, line, column)| format!("{file}:{line}:{column}"),
    );
    let payload = payload.map_or_else(|| "formatted".to_owned(), str::to_owned);
    let detail = format!("thread={thread} at={at} payload={payload}");
    let mut line = Line::new(Level::Error, "panic", "panic")
        .build(build)
        .code(PANIC)
        .detail(&detail);
    if let Some(trace) = trace {
        line = line.trace(trace);
    }
    line.render(process, unix_ms())
}

/// Install the hook at the start of `main`. It replaces the default hook and does not chain to
/// it, because the default prints the unredacted payload. It only writes: the panic proceeds
/// under the existing profile.
pub fn install_panic_hook(process: &'static str, build: String, trace: Option<String>) {
    std::panic::set_hook(Box::new(move |info| {
        let payload = info.payload().downcast_ref::<&'static str>().copied();
        let location = info
            .location()
            .map(|location| (location.file(), location.line(), location.column()));
        let thread = std::thread::current();
        eprintln!(
            "{}",
            panic_line(
                process,
                &build,
                trace.as_deref(),
                thread.name().unwrap_or("unnamed"),
                location,
                payload,
            )
        );
    }));
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::line::parse_line;

    #[test]
    fn the_line_has_build_code_trace_and_location() {
        let text = panic_line(
            "p",
            "0.1.0+x",
            Some("t"),
            "main",
            Some(("src/a.rs", 3, 9)),
            Some("boom"),
        );
        let parsed = parse_line(&text).unwrap();
        assert_eq!(parsed.module, "panic");
        assert_eq!(parsed.event, "panic");
        assert_eq!(parsed.build.as_deref(), Some("0.1.0+x"));
        assert_eq!(parsed.code, Some(4001));
        assert_eq!(parsed.name.as_deref(), Some("PANIC"));
        assert_eq!(parsed.cat.as_deref(), Some("INTERNAL_UNAVAILABLE"));
        assert_eq!(parsed.trace.as_deref(), Some("t"));
        assert_eq!(
            parsed.detail.as_deref(),
            Some("thread=main at=src/a.rs:3:9 payload=boom")
        );
    }

    #[test]
    fn a_formatted_payload_is_withheld() {
        let text = panic_line("p", "b", None, "t", None, None);
        let parsed = parse_line(&text).unwrap();
        assert_eq!(
            parsed.detail.as_deref(),
            Some("thread=t at=unknown payload=formatted")
        );
        assert!(parsed.trace.is_none());
    }
}
