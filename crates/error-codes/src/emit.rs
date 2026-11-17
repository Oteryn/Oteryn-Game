//! Filtered diagnostic output and a thread-local capture sink for tests.

use crate::{Line, filter, unix_ms};
use std::cell::RefCell;
use std::io::Write as _;
use std::sync::OnceLock;

static OBSERVER: OnceLock<fn(&Line<'_>)> = OnceLock::new();

thread_local! {
    static CAPTURE: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

/// Apply the global `OTERYN_LOG` filter, then render and write one line to stderr.
pub fn emit(process: &str, line: &Line<'_>) {
    if let Some(observer) = OBSERVER.get() {
        observer(line);
    }
    if !filter().allows(line.level, line.module, line.code.is_some()) {
        return;
    }
    let rendered = line.render(process, unix_ms());
    let captured = CAPTURE.with(|sink| {
        let mut sink = sink.borrow_mut();
        if let Some(lines) = sink.as_mut() {
            lines.push(rendered.clone());
            true
        } else {
            false
        }
    });
    if !captured {
        let stderr = std::io::stderr();
        let mut stderr = stderr.lock();
        let _ = writeln!(stderr, "{rendered}");
    }
}

/// Register a process-wide observer invoked for every emitted line before filtering.
///
/// The first observer wins, so a process composition root can install its sink once.
pub fn install_observer(observer: fn(&Line<'_>)) {
    let _ = OBSERVER.set(observer);
}

/// Capture lines emitted on this thread while `operation` runs.
///
/// Nested capture replaces the outer sink temporarily and restores it afterwards.
pub fn capture<F: FnOnce()>(operation: F) -> Vec<String> {
    let previous = CAPTURE.with(|sink| sink.replace(Some(Vec::new())));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation));
    let lines = CAPTURE.with(|sink| sink.replace(previous));
    match result {
        Ok(()) => lines.unwrap_or_default(),
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Level;

    #[test]
    fn capture_collects_emitted_lines_without_stderr() {
        let lines = capture(|| emit("p", &Line::new(Level::Info, "test", "captured")));
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("module=test event=captured"));
    }

    #[test]
    fn capture_restores_its_sink_after_a_panic() {
        let _ = std::panic::catch_unwind(|| {
            capture(|| std::panic::resume_unwind(Box::new("test panic")));
        });
        let lines = capture(|| emit("p", &Line::new(Level::Info, "test", "after")));
        assert_eq!(lines.len(), 1);
    }
}
