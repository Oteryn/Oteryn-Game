//! Preserve each journey's concrete error shape while observing its typed source.
use super::*;

pub(crate) trait IndexedError {
    fn as_error(&self) -> &(dyn Error + 'static);
}

impl IndexedError for Box<dyn Error> {
    fn as_error(&self) -> &(dyn Error + 'static) {
        self.as_ref()
    }
}

impl<E: Error + 'static> IndexedError for Box<E> {
    fn as_error(&self) -> &(dyn Error + 'static) {
        self.as_ref()
    }
}

macro_rules! indexed_errors {
    ($($error:ty),* $(,)?) => {$(
        impl IndexedError for $error {
            fn as_error(&self) -> &(dyn Error + 'static) { self }
        }
    )*};
}
indexed_errors!(
    std::io::Error,
    oteryn_dev_client::DevClientError,
    oteryn_dev_client::SessionError,
    sqlx::Error
);

pub(crate) fn record_error(error: &(dyn Error + 'static)) {
    let watch = Harness::global().active().clone();
    if let Some(watch) = watch.filter(|watch| watch.scenario.is_some()) {
        watch.remember(error);
    }
}

/// A failed assertion is recorded when checked, before any later await/cleanup.
pub(crate) fn record_assertion(check: &str) {
    let watch = Harness::global().active().clone();
    if let Some(watch) = watch.filter(|watch| watch.scenario.is_some()) {
        let mut state = watch.state();
        if state.stall.is_none() {
            state.root = Some(format!("{}/{check}", watch.row));
            state.stall = Some(Cause::Assertion);
        }
    }
}

#[derive(Debug)]
struct ContextError {
    message: String,
    source: Box<dyn Error>,
}
impl fmt::Display for ContextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}
impl Error for ContextError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}

/// Text is presentation context; the typed source remains available to classify.
pub(crate) fn context(message: String, error: impl Into<Box<dyn Error>>) -> Box<dyn Error> {
    Box::new(ContextError {
        message,
        source: error.into(),
    })
}

/// No boxing/coercion of the result: reporters retain their concrete step fields.
pub(crate) async fn observe<T, E: IndexedError>(
    journey: impl Future<Output = Result<T, E>>,
) -> Result<T, E> {
    let result = journey.await;
    if let Err(error) = &result {
        record_error(error.as_error());
    }
    result
}

/// Preserve both error parameters, including Elapsed for callers' timeout arms.
/// Only a row's actual deadline is observed here; quiet negative probes continue
/// using their ordinary timeout API and are judged by their owning assertion.
pub(crate) async fn indexed_timeout<T, E: IndexedError>(
    budget: Duration,
    journey: impl Future<Output = Result<T, E>>,
) -> Result<Result<T, E>, tokio::time::error::Elapsed> {
    let result = tokio::time::timeout(budget, observe(journey)).await;
    if let Err(error) = &result {
        record_error(error);
    }
    result
}
