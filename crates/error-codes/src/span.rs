//! RAII operation spans which always emit one end line.

use crate::{ErrorCode, FieldValue, Level, Line, Progression, emit, filter};
use std::time::{Duration, Instant};

const MAX_FIELDS: usize = 16;

#[derive(Debug)]
enum OwnedField {
    Token(&'static str, String),
    Number(&'static str, u64),
}

/// An operation lifetime which emits a diagnostic end line when dropped.
#[derive(Debug)]
pub struct Span {
    process: String,
    requested_level: Level,
    module: &'static str,
    event: &'static str,
    trace: Option<String>,
    parent: Option<String>,
    started: Instant,
    slow_after: Option<Duration>,
    outcome: &'static str,
    code: Option<ErrorCode>,
    fields: Vec<OwnedField>,
    detail: Option<String>,
}

impl Span {
    /// Start a span and emit its debug start event when enabled.
    #[must_use]
    pub fn start(
        level: Level,
        module: &'static str,
        event: &'static str,
        trace: Option<&str>,
    ) -> Self {
        let process = process_name();
        if filter().allows(Level::Debug, module, false) {
            let start_event = format!("{event}_start");
            let mut line = Line::new(Level::Debug, module, &start_event);
            if let Some(trace) = trace {
                line = line.trace(trace);
            }
            emit(&process, &line);
        }
        Self {
            process,
            requested_level: level,
            module,
            event,
            trace: trace.map(str::to_owned),
            parent: None,
            started: Instant::now(),
            slow_after: None,
            outcome: "unset",
            code: None,
            fields: Vec::new(),
            detail: None,
        }
    }

    /// Set the parent trace token.
    #[must_use]
    pub fn parent(mut self, trace: &str) -> Self {
        self.parent = Some(trace.to_owned());
        self
    }

    /// Promote a successful end to warning when this duration is exceeded.
    #[must_use]
    pub fn slow_after(mut self, duration: Duration) -> Self {
        self.slow_after = Some(duration);
        self
    }

    /// Set the terminal outcome token.
    pub fn set_outcome(&mut self, outcome: &'static str) {
        self.outcome = outcome;
    }

    /// Set the registered terminal error code.
    pub fn set_code(&mut self, code: ErrorCode) {
        self.code = Some(code);
    }

    /// Add one token field. At most sixteen extension fields are retained.
    pub fn field(&mut self, key: &'static str, value: impl Into<SpanField>) {
        if self.fields.len() >= MAX_FIELDS {
            return;
        }
        match value.into() {
            SpanField::Token(mut value) => {
                truncate_utf8(&mut value, 128);
                self.fields.push(OwnedField::Token(key, value));
            }
            SpanField::Number(value) => self.fields.push(OwnedField::Number(key, value)),
        }
    }

    /// Set redacted detail text. Rendering applies the diagnostic detail bound and escaping.
    pub fn set_detail(&mut self, detail: String) {
        let mut detail = detail;
        truncate_utf8(&mut detail, 512);
        self.detail = Some(detail);
    }

    /// Start a child span with a caller-provided trace and this span's trace as its parent.
    #[must_use]
    pub fn child_with_trace(&self, module: &'static str, event: &'static str, trace: &str) -> Self {
        let mut child = Self::start(self.requested_level, module, event, Some(trace));
        child.parent = self.trace.clone();
        child
    }
}

/// A token or number accepted by [`Span::field`].
#[derive(Debug)]
pub enum SpanField {
    /// A token value.
    Token(String),
    /// An unsigned numeric value.
    Number(u64),
}

impl From<&str> for SpanField {
    fn from(value: &str) -> Self {
        Self::Token(value.to_owned())
    }
}

impl From<String> for SpanField {
    fn from(value: String) -> Self {
        Self::Token(value)
    }
}

impl From<u64> for SpanField {
    fn from(value: u64) -> Self {
        Self::Number(value)
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        let elapsed = self.started.elapsed();
        let milliseconds = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
        let slow = self
            .slow_after
            .is_some_and(|threshold| elapsed >= threshold);
        let level = if self.outcome == "unset" && cfg!(debug_assertions) {
            Level::Error
        } else if let Some(code) = self.code {
            if code.progression() == Progression::Retryable {
                Level::Warn
            } else {
                Level::Error
            }
        } else if slow {
            Level::Warn
        } else {
            Level::Info
        };
        if !filter().allows(level, self.module, self.code.is_some()) {
            return;
        }
        let end_event = format!("{}_end", self.event);
        let mut line = Line::new(level, self.module, &end_event)
            .field("outcome", FieldValue::Token(self.outcome))
            .field("ms", FieldValue::U64(milliseconds));
        if let Some(code) = self.code {
            line = line.code(code);
        }
        if let Some(trace) = self.trace.as_deref() {
            line = line.trace(trace);
        }
        if let Some(parent) = self.parent.as_deref() {
            line = line.field("parent", FieldValue::Token(parent));
        }
        if slow {
            line = line.field("slow", FieldValue::Token("true"));
        }
        for field in &self.fields {
            line = match field {
                OwnedField::Token(key, value) => line.field(key, FieldValue::Token(value)),
                OwnedField::Number(key, value) => line.field(key, FieldValue::U64(*value)),
            };
        }
        if let Some(detail) = self.detail.as_deref() {
            line = line.detail(detail);
        }
        emit(&self.process, &line);
    }
}

/// Assert that captured lines contain no span with an unset outcome.
pub fn assert_no_unset(lines: &[String]) {
    assert!(
        lines.iter().all(|line| !line.contains(" outcome=unset")),
        "captured span ended with outcome=unset"
    );
}

fn process_name() -> String {
    std::env::args_os()
        .next()
        .and_then(|path| {
            std::path::PathBuf::from(path)
                .file_name()
                .map(|name| name.to_owned())
        })
        .and_then(|name| name.into_string().ok())
        .unwrap_or_else(|| "oteryn".to_owned())
}

fn truncate_utf8(value: &mut String, maximum: usize) {
    if value.len() <= maximum {
        return;
    }
    let boundary = value
        .char_indices()
        .map(|(index, _)| index)
        .take_while(|index| *index <= maximum)
        .last()
        .unwrap_or(0);
    value.truncate(boundary);
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::{Category, Progression, capture, parse_line};
    use std::future::Future;
    use std::pin::pin;
    use std::task::{Context, Waker};

    const RETRYABLE: ErrorCode =
        ErrorCode::new(2998, "RETRY", Category::Timeout, Progression::Retryable);

    fn early_return() {
        let mut span = Span::start(Level::Info, "test", "early", Some("trace"));
        span.set_outcome("returned");
    }

    fn question_mark() -> Result<(), ()> {
        let mut span = Span::start(Level::Info, "test", "question", None);
        let result: Result<(), ()> = Err(());
        result?;
        span.set_outcome("ok");
        Ok(())
    }

    #[test]
    fn end_is_written_for_early_return_question_mark_and_cancelled_future() {
        let lines = capture(|| {
            early_return();
            let _ = question_mark();
            let mut cancelled = pin!(async {
                let _span = Span::start(Level::Info, "test", "cancelled", None);
                std::future::pending::<()>().await;
            });
            let waker = Waker::noop();
            let mut context = Context::from_waker(waker);
            assert!(cancelled.as_mut().poll(&mut context).is_pending());
        });
        assert_eq!(lines.len(), 3);
        assert!(
            lines
                .iter()
                .any(|line| line.contains("event=early_end") && line.contains("outcome=returned"))
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("event=question_end outcome=unset"))
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("event=cancelled_end outcome=unset"))
        );
    }

    #[test]
    fn slow_and_retryable_ends_are_warnings() {
        let lines = capture(|| {
            let mut slow =
                Span::start(Level::Debug, "test", "slow", None).slow_after(Duration::ZERO);
            slow.set_outcome("ok");
            drop(slow);
            let mut retry = Span::start(Level::Info, "test", "retry", None);
            retry.set_outcome("failed");
            retry.set_code(RETRYABLE);
        });
        assert!(lines[0].contains("level=warn") && lines[0].contains("slow=true"));
        assert!(lines[1].contains("level=warn") && lines[1].contains("code=E2998"));
    }

    #[test]
    fn unset_detection_and_detail_escaping_apply() {
        let lines = capture(|| {
            let mut span = Span::start(Level::Info, "test", "detail", None);
            span.set_outcome("failed");
            span.set_detail("line one\nline two \"quoted\"".to_owned());
        });
        assert_no_unset(&lines);
        let parsed = parse_line(&lines[0]).unwrap();
        assert_eq!(
            parsed.detail.as_deref(),
            Some("line one\nline two \"quoted\"")
        );
        assert!(
            std::panic::catch_unwind(|| assert_no_unset(&["p outcome=unset".to_owned()])).is_err()
        );
    }
}
