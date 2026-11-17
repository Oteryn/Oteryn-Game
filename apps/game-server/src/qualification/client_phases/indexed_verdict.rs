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
        let mut pending = std::mem::take(&mut self.state().pending);
        if let Some(capture_failure) = pending.iter().position(|line| is_capture_write_error(line))
        {
            let failure = pending
                .iter()
                .position(|line| line.contains(" status=FAIL ") && !is_capture_write_error(line))
                .unwrap_or(capture_failure);
            let terminal = pending.swap_remove(failure);
            pending.clear();
            pending.push(terminal);
        }
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

fn is_capture_write_error(line: &str) -> bool {
    line.split_whitespace().any(|field| {
        field == "evidence=CAPTURE-WRITE-ERROR"
            || field.starts_with("evidence=CAPTURE-WRITE-ERROR;")
    })
}

fn terminal_capture(scenario: &str) -> bool {
    matches!(
        scenario,
        "C02.client"
            | "C08.client"
            | "C17c.client"
            | "C19a.client"
            | "E01.client"
            | "E02.client"
            | "E03.client"
            | "E05.client"
            | "E06.client"
            | "E07.client"
            | "E08.client"
            | "E09.client"
            | "E10.client"
            | "E12.client"
    )
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
pub(crate) fn indexed_evidence(mut line: String) -> Option<String> {
    if !(line.starts_with("e2e_all ")
        || line.starts_with("e2e_capture ")
        || line.contains("_CHECK ") && line.contains(" PASS "))
    {
        return Some(line);
    }
    let harness = Harness::global();
    let watch = harness.active().clone();
    let Some(watch) = watch.filter(|watch| watch.scenario.is_some()) else {
        for scenario in ["C14.full", "C19a.full", "C20b.full", "C20c.full"] {
            if line.starts_with(&format!("e2e_all scenario={scenario} status=PASS ")) {
                harness
                    .completed_prerequisites
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .insert(scenario);
                break;
            }
        }
        return Some(line);
    };
    if let Some(scenario) = watch.scenario.filter(|scenario| terminal_capture(scenario))
        && let Some(detail) =
            line.strip_prefix(&format!("e2e_capture scenario={scenario} status=CAPTURED"))
    {
        line = crate::qualification::e2e_result_line(&format!(
            "e2e_all scenario={scenario} status=PASS{detail}"
        ));
    }
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

#[cfg(test)]
mod capture_tests {
    use super::*;

    #[test]
    fn indexed_capture_completion_is_a_terminal_pass() {
        let mut watch = Watch::new("C02.client", Harness::global());
        watch.scenario = Some("C02.client");
        let active = watch.enter();

        assert_eq!(
            indexed_evidence("e2e_capture scenario=C02.client status=CAPTURED attack=1".to_owned()),
            None
        );
        drop(active);
        assert_eq!(
            watch.take_verdicts(),
            ["e2e_all scenario=C02.client status=PASS seconds=0.000 evidence=attack=1"]
        );
    }

    #[test]
    fn capture_write_failure_has_one_terminal_verdict() {
        let mut watch = Watch::new("I03.client", Harness::global());
        watch.scenario = Some("I03.client");
        watch.state().pending.extend([
            "e2e_all scenario=I03.client status=FAIL seconds=0.000 evidence=CAPTURE-WRITE-ERROR;disk".to_owned(),
            "e2e_all scenario=I03.client status=PASS seconds=0.000".to_owned(),
        ]);

        let verdicts = watch.take_verdicts();
        assert_eq!(verdicts.len(), 1);
        assert!(verdicts[0].contains("status=FAIL"));
        assert!(verdicts[0].contains("CAPTURE-WRITE-ERROR;disk"));
    }
}
