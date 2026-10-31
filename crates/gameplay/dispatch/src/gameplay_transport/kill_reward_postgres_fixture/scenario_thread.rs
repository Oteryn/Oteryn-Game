//! The existing composed-scenario thread, extracted for other real-dispatch scenarios.
use super::TestResult;

pub(crate) fn run(
    name: &str,
    scenario: impl FnOnce() -> TestResult + Send + 'static,
) -> TestResult {
    std::thread::Builder::new()
        .name(name.into())
        .stack_size(64 << 20)
        .spawn(move || scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "composed PostgreSQL scenario thread panicked")?
        .map_err(Into::into)
}

/// Construct and poll the complete PostgreSQL future on the scenario stack.
pub(crate) fn run_async<F>(body: F) -> TestResult
where
    F: AsyncFnOnce(String) -> TestResult + Send + 'static,
{
    run("composed-postgres", move || {
        super::super::fixture::run(body)
    })
}
