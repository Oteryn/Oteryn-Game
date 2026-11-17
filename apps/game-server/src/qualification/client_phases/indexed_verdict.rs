//! Typed boundary for indexed reporters that keep their own action assertions.
use super::*;

impl Watch {
    pub(super) fn remember(&self, error: &(dyn Error + 'static)) {
        let cause = classify(error);
        let mut state = self.state();
        if state.stall.is_none() {
            state.root = Some(match &cause {
                Cause::BlockedBy { root } => root.clone(),
                Cause::Precondition { name } => format!("{}/{name}", self.row),
                _ => format!("{}/{}", self.row, state.phase),
            });
            state.stall = Some(cause);
        }
    }

    fn failure(&self) -> (Cause, String) {
        let state = self.state();
        let cause = state.stall.clone().unwrap_or(Cause::Assertion);
        let root = state.root.clone().unwrap_or_else(|| match &cause {
            Cause::BlockedBy { root } => root.clone(),
            Cause::Precondition { name } => format!("{}/{name}", self.row),
            _ => format!("{}/{}", self.row, state.phase),
        });
        (cause, root)
    }

    fn ensure_failure_verdict(&self) {
        let mut state = self.state();
        if state.stall.is_some() && state.pending.is_empty() {
            let scenario = self.scenario.unwrap_or(self.row);
            state.pending.push(format!(
                "e2e_all scenario={scenario} status=FAIL seconds=0.000 evidence=observed-error-without-verdict"
            ));
        }
    }

    fn take_verdicts(&self) -> Vec<String> {
        let pending = std::mem::take(&mut self.state().pending);
        let mut lines = Vec::with_capacity(pending.len());
        for mut line in pending {
            // An error during cleanup cannot leave an earlier optimistic PASS.
            let state = self.state();
            let failed_exit = state.failed_exit;
            let failed_pass = state.stall.is_some() && line.contains(" status=PASS ");
            drop(state);
            if failed_exit || failed_pass {
                for status in ["PASS", "SKIPPED", "MISSING-SERVER", "MISSING-CLIENT"] {
                    line = line.replacen(&format!(" status={status} "), " status=FAIL ", 1);
                }
            }
            if line.contains(" status=FAIL ") && !line.contains(" cause=") {
                let (cause, root) = self.failure();
                evidence(&format!(
                    "QUALIFICATION_CAUSE row={} cause={} root={root}{}",
                    self.row,
                    cause.kind(),
                    cause.fields()
                ));
                self.harness.fail(Need::Row(self.row), root.clone());
                line.push_str(&cause.suffix(&root));
            }
            lines.push(line);
        }
        lines
    }
}

/// Enroll the dispatch boundary without changing any row's coverage assertions.
pub(crate) fn indexed(
    scenario: &'static str,
    journey: impl Future<Output = TestResult>,
) -> impl Future<Output = ()> {
    // Each row owns its future on the heap; hundreds of inline row temporaries
    // otherwise accumulate in journeys' debug-build poll frame.
    Box::pin(async move {
        let mut watch = Watch::new(scenario, Harness::global());
        watch.scenario = Some(scenario);
        let result = watch.run(None, "indexed row", journey).await;
        if let Err(error) = result {
            watch.remember(error.as_ref());
            watch.state().failed_exit = true;
            // The cancelled reporter did not get to serialize its typed failure.
            if watch.state().pending.is_empty() {
                watch.state().pending.push(format!(
                "e2e_all scenario={scenario} status=FAIL seconds=0.000 evidence=watch-aborted-before-verdict"
            ));
            }
        }
        watch.ensure_failure_verdict();
        // `run` restored the previous active Watch, so flushing cannot recapture itself.
        for line in watch.take_verdicts() {
            evidence(&line);
        }
    })
}

/// Called after legacy line normalization, before output. Buffer only this
/// indexed row's verdict so late cleanup failures retain their typed origin.
pub(crate) fn indexed_evidence(line: String) -> Option<String> {
    if !line.starts_with("e2e_all ") && !(line.contains("_CHECK ") && line.contains(" PASS ")) {
        return Some(line);
    }
    let watch = Harness::global().active().clone();
    let Some(watch) = watch.filter(|watch| watch.scenario.is_some()) else {
        return Some(line);
    };
    if line.contains("_CHECK ") && line.contains(" PASS ") {
        watch.touch("row-check");
    }
    if watch
        .scenario
        .is_some_and(|scenario| line.starts_with(&format!("e2e_all scenario={scenario} ")))
    {
        watch.state().pending.push(line);
        None
    } else {
        Some(line)
    }
}

impl Harness {
    /// A cached shared observation keeps the first row's typed root on replay.
    pub(crate) fn shared_failure(&self, key: &'static str) {
        let watch = self.active().clone();
        if let Some(watch) = watch.filter(|watch| watch.scenario.is_some()) {
            let root = self.health().get(&Need::Row(key)).cloned();
            if let Some(root) = root {
                watch.remember(
                    HarnessError::boxed(Cause::BlockedBy { root }, "cached observation failed")
                        .as_ref(),
                );
            } else {
                self.fail(Need::Row(key), watch.failure().1);
            }
        }
    }

    /// A condition/timer with its own existing deadline is not a stalled row.
    /// This never changes that deadline or the enclosing journey's wall budget.
    pub(crate) fn bounded_wait(&self, until: tokio::time::Instant, phase: &'static str) {
        if let Some(watch) = self
            .active()
            .as_ref()
            .filter(|watch| watch.scenario.is_some())
        {
            let mut state = watch.state();
            state.phase = phase;
            state.last = tokio::time::Instant::now();
            state.idle_until = Some(until);
            state.wakeup.notify_one();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime() -> TestResult<tokio::runtime::Runtime> {
        Ok(tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .start_paused(true)
            .build()?)
    }

    fn new_watch(harness: &'static Harness) -> Watch {
        let mut watch = Watch::new("C03.full", harness);
        watch.scenario = Some("C03.full");
        watch
    }

    #[test]
    fn typed_first_error_and_originating_root_survive_cleanup() -> TestResult {
        runtime()?.block_on(async {
            let harness = Box::leak(Box::new(Harness::new(true, None)));
            let watch = new_watch(harness);
            let active = watch.enter();
            let first = harness.fixture_failed(1, &std::io::Error::other("release expired"));
            watch.remember(first.as_ref());
            watch.remember(&std::io::Error::other("cleanup transport closed"));
            let (cause, root) = watch.failure();
            assert_eq!(
                cause,
                Cause::Precondition {
                    name: "fixture1-released".to_owned()
                }
            );
            assert_eq!(root, "C03.full/fixture1-released");
            watch.ensure_failure_verdict();
            let missing = watch.take_verdicts();
            assert_eq!(missing.len(), 1);
            assert!(missing[0].contains("status=FAIL"));
            assert!(missing[0].contains("root=C03.full/fixture1-released"));
            watch.state().pending.push("e2e_all scenario=C03.full status=PASS seconds=1".to_owned());
            watch.state().failed_exit = true;
            let lines = watch.take_verdicts();
            assert_eq!(lines, vec!["e2e_all scenario=C03.full status=FAIL seconds=1 cause=precondition root=C03.full/fixture1-released"]);
            drop(active);
            let second = new_watch(harness);
            let blocked = harness.fixture_gate(1);
            let error = blocked.as_ref().err().ok_or("expected fixture block")?;
            second.remember(error.as_ref());
            assert_eq!(second.failure().0, Cause::BlockedBy { root });
            Ok(())
        })
    }

    #[test]
    fn bounded_observation_wait_preserves_its_real_budget_and_idle_watchdog() -> TestResult {
        runtime()?.block_on(async {
            let harness = Box::leak(Box::new(Harness::new(true, Some(Duration::from_secs(90)))));
            let watch = new_watch(harness);
            let started = tokio::time::Instant::now();
            watch
                .bound(Duration::from_secs(180), "real journey deadline", async {
                    harness.bounded_wait(
                        tokio::time::Instant::now() + Duration::from_secs(130),
                        "soul-regeneration",
                    );
                    tokio::time::sleep(Duration::from_secs(130)).await;
                    Ok(())
                })
                .await?;
            assert_eq!(started.elapsed(), Duration::from_secs(130));
            let release = new_watch(harness);
            let started = tokio::time::Instant::now();
            let error: TestResult = release
                .bound(Duration::from_secs(180), "release wall deadline", async {
                    harness.bounded_wait(
                        tokio::time::Instant::now() + Duration::from_secs(160),
                        "fixture-release",
                    );
                    tokio::time::sleep(Duration::from_secs(160)).await;
                    // The bounded diagnostic after release expiry may finish before
                    // fixture_failed records health and its originating row.
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    Err(harness.fixture_failed(0, &std::io::Error::other("release expired")))
                })
                .await;
            assert_eq!(started.elapsed(), Duration::from_secs(162));
            assert!(matches!(
                error.as_ref().err().map(|error| classify(error.as_ref())),
                Some(Cause::Precondition { .. })
            ));
            assert!(harness.fixture_gate(0).is_err());
            let idle = new_watch(harness);
            let error = idle
                .run(None, "no wall bound", std::future::pending::<TestResult>())
                .await;
            assert!(matches!(
                error.as_ref().err().map(|error| classify(error.as_ref())),
                Some(Cause::NoProgress { .. })
            ));
            let deadline = new_watch(harness);
            let error = deadline
                .bound(Duration::from_secs(10), "real deadline", async {
                    harness.bounded_wait(
                        tokio::time::Instant::now() + Duration::from_secs(780),
                        "offline-recovery",
                    );
                    std::future::pending::<TestResult>().await
                })
                .await;
            assert!(matches!(
                error.as_ref().err().map(|error| classify(error.as_ref())),
                Some(Cause::Deadline { .. })
            ));
            Ok(())
        })
    }
}
