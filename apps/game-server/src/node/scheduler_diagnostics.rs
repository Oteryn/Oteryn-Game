use oteryn_error_codes::{Level, Span};

oteryn_error_codes::error_kinds! {
    /// A bounded scheduler pass could not complete and will be retried.
    pub(super) enum SchedulerErrorKind {
        JobFailed = (2015, "SCHEDULER_JOB_FAILED", InternalUnavailable, Retryable),
    }
}

pub(super) fn job(name: &'static str) -> Span {
    let mut span = Span::start(Level::Info, "scheduler", "job", None);
    span.field("name", name);
    span
}
