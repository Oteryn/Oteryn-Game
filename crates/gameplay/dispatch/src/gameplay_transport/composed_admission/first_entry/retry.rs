use super::*;

/// Diagnostic state only; the lock is never held across an await.
pub(super) struct EntryPhase(std::sync::Mutex<&'static str>);

impl Default for EntryPhase {
    fn default() -> Self {
        Self(std::sync::Mutex::new("resolve_session"))
    }
}

impl EntryPhase {
    pub(super) fn set(&self, phase: &'static str) {
        *self.0.lock().unwrap_or_else(|error| error.into_inner()) = phase;
    }

    pub(super) fn current(&self) -> &'static str {
        *self.0.lock().unwrap_or_else(|error| error.into_inner())
    }
}

pub(super) type EntryResult = (
    FirstEntryOutcome,
    Option<CurrentCharacterItemFence>,
    Option<last_position::AdmissionPosition>,
);

pub(super) async fn enter_with_retry(
    budget: &AdmissionRetryBudget<'_>,
    phase: &EntryPhase,
    mut initialize: impl AsyncFnMut() -> EntryResult,
) -> EntryResult {
    let started = tokio::time::Instant::now();
    let mut attempts = 0_u64;
    let mut stale_rounds = 1;
    let result = loop {
        attempts = attempts.saturating_add(1);
        let result = initialize().await;
        match result.0 {
            FirstEntryOutcome::RefusedUnavailable
                if result.2.is_none()
                    && budget
                        .retries_unavailable(AdmissionRefusal::Unavailable)
                        .await => {}
            FirstEntryOutcome::RefusedStaleAuthority if stale_rounds < RECONCILE_ATTEMPTS => {
                stale_rounds += 1;
                tokio::time::sleep(RECONCILE_BACKOFF).await;
            }
            _ => break result,
        }
    };
    let outcome = match result.0 {
        FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled => "playable",
        FirstEntryOutcome::RefusedUnavailable | FirstEntryOutcome::RefusedStaleAuthority => {
            "retries_exhausted"
        }
        _ => "unplayable",
    };
    record_entry_decision(outcome, attempts, started.elapsed(), phase.current());
    result
}

#[cfg(test)]
mod diagnostic_tests {
    use super::*;

    #[test]
    fn first_entry_decision_reports_unplayable_and_exhausted_retries()
    -> Result<(), Box<dyn std::error::Error>> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let mut session_bytes = [1; 16];
        session_bytes[6] = 0x71;
        session_bytes[8] = 0x91;
        let session = GameSessionId::decode(&session_bytes)?;
        let lines = oteryn_error_codes::capture(|| {
            runtime.block_on(async {
                tokio::time::pause();
                for outcome in [
                    FirstEntryOutcome::RefusedByChannel,
                    FirstEntryOutcome::RefusedStaleAuthority,
                    FirstEntryOutcome::RefusedUnavailable,
                ] {
                    let budget = AdmissionRetryBudget::new(&session);
                    let phase = EntryPhase::default();
                    let mut calls = 0_u32;
                    let result = enter_with_retry(&budget, &phase, async || {
                        calls += 1;
                        (outcome, None, None)
                    })
                    .await;
                    assert_eq!(result.0, outcome);
                    if outcome == FirstEntryOutcome::RefusedStaleAuthority {
                        assert_eq!(calls, RECONCILE_ATTEMPTS);
                    }
                    assert!(calls > 0);
                }
            })
        });
        assert_eq!(lines.len(), 3);
        for (line, outcome) in
            lines
                .iter()
                .zip(["unplayable", "retries_exhausted", "retries_exhausted"])
        {
            assert!(line.contains("level=info"), "{line}");
            assert!(line.contains("event=admission_entry_decision"), "{line}");
            assert!(line.contains(&format!("outcome={outcome}")), "{line}");
            assert!(line.contains("phase=resolve_session"), "{line}");
            assert!(line.contains("elapsed_ms="), "{line}");
            assert!(line.contains("attempts="), "{line}");
        }
        assert!(lines[0].contains("attempts=1"));
        assert!(lines[1].contains("attempts=5"));
        Ok(())
    }
}
