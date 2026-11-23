//! Familiar Login runs after placement, so its retries stay within that step.

pub(super) async fn run<E>(
    mut initialize: impl AsyncFnMut() -> Result<(), E>,
    retryable: impl Fn(&E) -> bool,
    mut wait: impl AsyncFnMut() -> bool,
    mut report_failure: impl FnMut(&E, u64),
) -> (Result<(), E>, u64) {
    let mut attempts = 0_u64;
    loop {
        attempts = attempts.saturating_add(1);
        match initialize().await {
            Ok(()) => return (Ok(()), attempts),
            Err(error) => {
                report_failure(&error, attempts);
                if !retryable(&error) || !wait().await {
                    return (Err(error), attempts);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::run;
    use std::future::{Future, ready};
    use std::task::{Context, Poll, Waker};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Failure {
        Unavailable,
        Refused,
    }

    fn decide(
        first: Failure,
        retry_allowed: bool,
    ) -> Result<(Result<(), Failure>, u64, u64), &'static str> {
        let mut attempts = 0;
        let mut waits = 0;
        let mut failures = Vec::new();
        let decision = {
            let mut decision = std::pin::pin!(run(
                async || {
                    attempts += 1;
                    if attempts == 1 { Err(first) } else { Ok(()) }
                },
                |failure| *failure == Failure::Unavailable,
                || {
                    waits += 1;
                    ready(retry_allowed)
                },
                |error, attempt| failures.push((*error, attempt)),
            ));
            decision
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
        };
        let Poll::Ready((outcome, reported_attempts)) = decision else {
            return Err("ready test steps must complete in one poll");
        };
        assert_eq!(failures, [(first, 1)]);
        assert_eq!(reported_attempts, attempts);
        Ok((outcome, attempts, waits))
    }

    #[test]
    fn familiar_login_transient_failure_retries_and_becomes_playable() -> Result<(), &'static str> {
        assert_eq!(decide(Failure::Unavailable, true)?, (Ok(()), 2, 1));
        Ok(())
    }

    #[test]
    fn familiar_login_permanent_refusal_stays_refused() -> Result<(), &'static str> {
        assert_eq!(
            decide(Failure::Refused, true)?,
            (Err(Failure::Refused), 1, 0)
        );
        Ok(())
    }

    #[test]
    fn familiar_login_stops_when_retry_budget_expires() -> Result<(), &'static str> {
        assert_eq!(
            decide(Failure::Unavailable, false)?,
            (Err(Failure::Unavailable), 1, 1)
        );
        Ok(())
    }
}
