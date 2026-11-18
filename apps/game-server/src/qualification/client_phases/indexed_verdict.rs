//! Typed boundary for indexed reporters that keep their own action assertions.
use super::*;

impl Watch {
    async fn run_indexed(&self, journey: impl Future<Output = TestResult>) -> TestResult {
        self.bound(Duration::from_secs(2400), "indexed row deadline", journey)
            .await
    }

    pub(super) fn remember(&self, error: &(dyn Error + 'static)) {
        let cause = classify(error);
        let mut state = self.state();
        if state.stall.is_none() {
            state.root = Some(match &cause {
                Cause::BlockedBy { root } => root.clone(),
                Cause::Precondition { name } => format!("{}/{name}", self.row),
                _ => format!(
                    "{}/{}",
                    self.row,
                    state.phase.replace(char::is_whitespace, "-")
                ),
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
            _ => format!(
                "{}/{}",
                self.row,
                state.phase.replace(char::is_whitespace, "-")
            ),
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
        if state.pending.is_empty() {
            let scenario = self.scenario.unwrap_or(self.row);
            state.stall = Some(Cause::Assertion);
            state.pending.push(format!(
                "e2e_all scenario={scenario} status=FAIL seconds=0.000 evidence=missing-final-verdict"
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
            lines.push(advisory_line(line, self.advisory));
        }
        lines
    }
}

fn indexed_completion(
    scenario: &str,
    result: TestResult,
    has_verdict: bool,
    report_success: bool,
    seconds: f64,
) -> (TestResult, Option<String>) {
    let line = if result.is_err() && !has_verdict {
        Some(format!(
            "e2e_all scenario={scenario} status=FAIL seconds=0.000 evidence=watch-aborted-before-verdict"
        ))
    } else if result.is_ok() && !has_verdict && report_success {
        Some(format!(
            "e2e_all scenario={scenario} status=PASS seconds={seconds:.3}"
        ))
    } else {
        None
    };
    (result, line)
}

#[cfg(test)]
mod completion_tests {
    use super::*;

    #[test]
    fn indexed_watchdog_failure_reaches_the_journey_caller() {
        let (result, line) =
            indexed_completion("CS.rat.full", Err("watchdog".into()), false, true, 0.0);
        assert!(line.is_some_and(|line| line.contains("status=FAIL")));
        assert!(result.is_err());
    }

    #[test]
    fn completed_species_gets_its_full_pass_verdict() {
        let scenario = "CS.rat.full";
        let (result, line) = indexed_completion(scenario, Ok(()), false, true, 1.0);
        assert!(result.is_ok());
        assert_eq!(
            line.as_deref(),
            Some("e2e_all scenario=CS.rat.full status=PASS seconds=1.000")
        );
        let (_, duplicate) = indexed_completion(scenario, Ok(()), true, true, 1.0);
        assert!(duplicate.is_none());
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

#[cfg(test)]
async fn run_indexed(watch: &Watch, journey: impl Future<Output = TestResult>) -> TestResult {
    watch.run_indexed(journey).await
}

/// Enroll the dispatch boundary without changing any row's coverage assertions.
pub(crate) fn indexed(
    scenario: &'static str,
    journey: impl Future<Output = TestResult>,
) -> impl Future<Output = bool> {
    Box::pin(async move {
        indexed_run(Harness::global(), scenario, journey, false, false)
            .await
            .is_ok()
    })
}

/// A checked row reports completion to its caller as well as to evidence consumers.
pub(crate) fn indexed_checked(
    scenario: &'static str,
    journey: impl Future<Output = TestResult>,
) -> impl Future<Output = TestResult> {
    indexed_checked_with(Harness::global(), scenario, journey)
}

pub(crate) fn indexed_advisory(
    scenario: &'static str,
    journey: impl Future<Output = TestResult>,
) -> impl Future<Output = TestResult> {
    indexed_run(Harness::global(), scenario, journey, true, true)
}

fn indexed_checked_with(
    harness: &'static Harness,
    scenario: &'static str,
    journey: impl Future<Output = TestResult>,
) -> impl Future<Output = TestResult> {
    indexed_run(harness, scenario, journey, true, false)
}

fn indexed_run(
    harness: &'static Harness,
    scenario: &'static str,
    journey: impl Future<Output = TestResult>,
    report_success: bool,
    advisory: bool,
) -> impl Future<Output = TestResult> {
    // Keep the row on the heap to limit the parent journey's debug-build poll frame.
    Box::pin(crate::qualification::client_world::FIXTURE_WORKERS.scope(
        Default::default(),
        async move {
            let started = std::time::Instant::now();
            let mut watch = Watch::new(scenario, harness);
            watch.scenario = Some(scenario);
            watch.advisory = advisory;
            let mut result = watch.run_indexed(journey).await;
            if report_success && result.is_ok() {
                let state = watch.state();
                let cause = state.stall.clone().or_else(|| {
                    state
                        .pending
                        .iter()
                        .any(|line| line.contains(" status=FAIL "))
                        .then_some(Cause::Assertion)
                });
                if let Some(cause) = cause {
                    result = Err(HarnessError::boxed(
                        cause,
                        format!("{scenario} reported failure"),
                    ));
                }
            }
            if let Err(error) = &result {
                watch.remember(error.as_ref());
                watch.state().failed_exit = true;
            }
            let has_verdict = !watch.state().pending.is_empty();
            let (result, missing) = indexed_completion(
                scenario,
                result,
                has_verdict,
                report_success,
                started.elapsed().as_secs_f64(),
            );
            if let Some(line) = missing {
                watch.state().pending.push(line);
            }
            watch.ensure_failure_verdict();
            // `run` restored the previous active Watch, so flushing cannot recapture itself.
            let lines = watch.take_verdicts();
            let all_passed = lines.iter().all(|line| line.contains(" status=PASS "));
            let mode = super::E2eMode::from_env()?.name();
            for line in lines {
                evidence(&format!("{line} mode={mode}"));
            }
            crate::qualification::client_world::await_fixture_workers().await;
            if !report_success && result.is_ok() && !all_passed {
                return Err(HarnessError::boxed(
                    Cause::Assertion,
                    format!("{scenario} reported failure"),
                ));
            }
            result
        },
    ))
}

/// Called after legacy line normalization, before output. Buffer only this
/// indexed row's verdict so late cleanup failures retain their typed origin.
pub(crate) fn indexed_evidence(line: String) -> Option<String> {
    indexed_evidence_for(Harness::global(), line)
}

fn indexed_evidence_for(harness: &Harness, mut line: String) -> Option<String> {
    if let Some((tag, _)) = line.split_once(' ')
        && let Some(row) = tag.strip_suffix("_CLIENT_CAPTURE_WRITE_ERROR")
        && let Some(watch) = harness.active().clone()
        && let Some(scenario) = watch.scenario.filter(|scenario| terminal_capture(scenario))
        && scenario
            .strip_suffix(".client")
            .is_some_and(|id| id.eq_ignore_ascii_case(row))
    {
        watch
            .state()
            .pending
            .push(crate::qualification::e2e_result_line(&format!(
                "e2e_all scenario={scenario} status=CAPTURE-WRITE-ERROR evidence={line}"
            )));
        return None;
    }
    if !(line.starts_with("e2e_all ")
        || line.starts_with("e2e_capture ")
        || line.contains("_CHECK ") && line.contains(" PASS "))
    {
        return Some(line);
    }
    let watch = harness.active().clone();
    let Some(watch) = watch.filter(|watch| watch.scenario.is_some()) else {
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
        Some(advisory_line(line, watch.advisory))
    }
}

fn advisory_line(line: String, advisory: bool) -> String {
    if advisory {
        line.replacen("e2e_all ", "CREATURE_SWEEP_CALIBRATION ", 1)
    } else {
        line
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
        if let Some(watch) = self.active().as_ref() {
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
    fn indexed_watchdog_cancellation_reaches_the_stage_caller() -> TestResult {
        runtime()?.block_on(async {
            let harness = Box::leak(Box::new(Harness::new(true, Some(Duration::from_secs(1)))));
            let result = indexed_checked_with(
                harness,
                "REVIEW.cancel.full",
                std::future::pending::<TestResult>(),
            )
            .await;
            assert!(result.is_err());
            assert!(
                harness
                    .blocked(&[Need::Row("REVIEW.cancel.full")])
                    .is_some()
            );
            Ok(())
        })
    }

    #[test]
    fn indexed_failure_reaches_the_stage_caller() -> TestResult {
        runtime()?.block_on(async {
            assert!(
                indexed_checked_with(
                    Box::leak(Box::new(Harness::new(true, None))),
                    "REVIEW.failure.full",
                    async { Err("scenario failed".into()) }
                )
                .await
                .is_err()
            );
            Ok(())
        })
    }

    #[test]
    fn qualification_indexed_progress_cannot_outlive_wall_deadline() -> TestResult {
        runtime()?.block_on(async {
            let harness = Box::leak(Box::new(Harness::new(true, None)));
            let watch = new_watch(harness);
            let journey = async {
                for _ in 0..=100 {
                    tokio::time::sleep(Duration::from_secs(30)).await;
                    watch.touch("active-search");
                }
                Ok(())
            };
            let error = run_indexed(&watch, journey)
                .await
                .err()
                .ok_or("indexed row completed past deadline")?;
            assert_eq!(
                classify(error.as_ref()),
                Cause::Deadline {
                    phase: "active-search"
                }
            );
            Ok(())
        })
    }

    #[test]
    fn qualification_indexed_deadline_precedes_fixture_cleanup() -> TestResult {
        use crate::qualification::client_world::{
            FIXTURE_WORKERS, FixtureCommand, await_fixture_workers,
        };
        runtime()?.block_on(FIXTURE_WORKERS.scope(Default::default(), async {
            let harness = Box::leak(Box::new(Harness::new(true, None)));
            let watch = new_watch(harness);
            let (release, gate) = std::sync::mpsc::sync_channel(1);
            let error = run_indexed(&watch, async {
                let _worker = FixtureCommand::start(move || {
                    gate.recv_timeout(Duration::from_secs(2))
                        .map_err(|error| error.to_string())
                })?;
                std::future::pending::<TestResult>().await
            })
            .await
            .err()
            .ok_or("missing row deadline")?;
            assert!(matches!(classify(error.as_ref()), Cause::Deadline { .. }));
            let mut cleanup = Box::pin(await_fixture_workers());
            tokio::select! {
                biased;
                () = &mut cleanup => return Err("worker escaped row cleanup".into()),
                () = tokio::task::yield_now() => {}
            }
            release.send(())?;
            cleanup.await;
            Ok(())
        }))
    }

    #[test]
    fn indexed_wall_clock_bounds_a_journey_reporting_progress() -> TestResult {
        runtime()?.block_on(async {
            let harness = Box::leak(Box::new(Harness::new(true, Some(Duration::from_secs(90)))));
            let watch = new_watch(harness);
            let outcome = tokio::time::timeout(
                Duration::from_secs(2401),
                watch.run_indexed(async {
                    loop {
                        tokio::time::sleep(Duration::from_secs(30)).await;
                        watch.touch("periodic-observation");
                    }
                }),
            )
            .await?;
            let error = outcome
                .err()
                .ok_or("indexed journey expired without a failure")?;
            assert!(matches!(classify(error.as_ref()), Cause::Deadline { .. }));
            Ok(())
        })
    }

    #[test]
    fn indexed_success_without_verdict_fails_closed() {
        let harness = Box::leak(Box::new(Harness::new(true, None)));
        let watch = new_watch(harness);
        watch.ensure_failure_verdict();
        let lines = watch.take_verdicts();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("status=FAIL"));
        assert!(lines[0].contains("missing-final-verdict"));
    }

    #[test]
    fn indexed_failure_root_is_one_token() {
        let harness = Box::leak(Box::new(Harness::new(true, None)));
        let watch = new_watch(harness);
        watch.touch("fixture provisioning liveness");
        watch.remember(&std::io::Error::other("fixture connection closed"));
        assert_eq!(watch.failure().1, "C03.full/fixture-provisioning-liveness");
    }

    #[test]
    fn typed_first_error_and_originating_root_survive_cleanup() -> TestResult {
        runtime()?.block_on(async {
            let harness = Box::leak(Box::new(Harness::new(true, None)));
            let watch = new_watch(harness);
            let active = watch.enter();
            let first = harness.fixture_failed(Need::Fixture([0; 16], [1; 16]), 1, &std::io::Error::other("release expired"));
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
            let blocked = harness.fixture_gate(Need::Fixture([0; 16], [1; 16]));
            let error = blocked.as_ref().err().ok_or("expected fixture block")?;
            second.remember(error.as_ref());
            assert_eq!(second.failure().0, Cause::BlockedBy { root });
            Ok(())
        })
    }

    #[test]
    fn rashid_indexed_wait_survives_watchdog_and_reports_the_wall_deadline() -> TestResult {
        use super::super::super::full_row_s10::{RashidWait, audit_rashid_anchors};

        runtime()?.block_on(async {
            let today = (30, 40, -7);
            for watchdog in [Some(Duration::from_secs(3)), None] {
                let harness = Box::leak(Box::new(Harness::new(true, watchdog)));
                let mut watch = Watch::new("S10.full", harness);
                watch.scenario = Some("S10.full");
                for appears in [true, false] {
                    let wait = RashidWait::new(harness, Duration::from_secs(12));
                    let mut probes = 0;
                    let result = watch
                        .run(
                            None,
                            "indexed row",
                            wait.run(audit_rashid_anchors(
                                &mut probes,
                                today,
                                [today],
                                true,
                                &wait,
                                async |probes, _| {
                                    *probes += 1;
                                    tokio::time::sleep(Duration::from_secs(1)).await;
                                    Ok((appears && *probes >= 8).then_some(today))
                                },
                                async |_, actor| Ok(actor),
                            )),
                        )
                        .await;
                    if appears {
                        assert_eq!(result?, today);
                        assert!(probes >= 8);
                    } else {
                        let error = result
                            .err()
                            .ok_or("missing Rashid should hit the wall deadline")?;
                        assert_eq!(
                            classify(error.as_ref()),
                            Cause::Deadline {
                                phase: "rashid-rotation"
                            }
                        );
                        assert!(error.to_string().contains("waiting for live Rashid"));
                    }
                }
            }
            Ok(())
        })
    }

    #[test]
    fn rashid_look_backoff_survives_route_progress_under_one_second_watchdog() -> TestResult {
        use super::super::super::full_row_s10::{
            RashidWait, audit_rashid_anchors, look_for_rashid,
        };
        use oteryn_protocol_oteryn::item_look::{ItemLookRefusal, ItemLookResult};

        runtime()?.block_on(async {
            let harness = Box::leak(Box::new(Harness::new(true, Some(Duration::from_secs(1)))));
            let mut watch = Watch::new("S10.full", harness);
            watch.scenario = Some("S10.full");
            let today = (30, 40, -7);
            let wait = RashidWait::new(harness, Duration::from_secs(12));
            let mut probes = 0;
            let result = watch
                .run(
                    None,
                    "indexed row",
                    wait.run(audit_rashid_anchors(
                        &mut probes,
                        today,
                        [today],
                        true,
                        &wait,
                        async |probes, _| {
                            *probes += 1;
                            if *probes == 1 {
                                return Ok(None);
                            }
                            if *probes == 2 {
                                harness.progress("route-search");
                                harness.progress("walk");
                                let mut replies = [
                                    ItemLookResult::Refused(ItemLookRefusal::RateLimited),
                                    ItemLookResult::Text("You see Rashid.".into()),
                                ]
                                .into_iter();
                                let found = look_for_rashid(
                                    &mut replies,
                                    today,
                                    &wait,
                                    async |replies, _| {
                                        replies.next().ok_or_else(|| "unexpected LOOK".into())
                                    },
                                    async |_, duration| {
                                        tokio::time::sleep(duration).await;
                                        Ok(())
                                    },
                                )
                                .await?;
                                return Ok(found.then_some(today));
                            }
                            Ok(Some(today))
                        },
                        async |_, actor| Ok(actor),
                    )),
                )
                .await?;
            assert_eq!(result, today);
            assert_eq!(probes, 3);
            Ok(())
        })
    }

    #[test]
    fn rashid_pending_look_is_subject_to_the_watchdog_after_an_empty_probe() -> TestResult {
        use super::super::super::full_row_s10::{RashidWait, look_for_rashid};

        runtime()?.block_on(async {
            let harness = Box::leak(Box::new(Harness::new(true, Some(Duration::from_secs(1)))));
            let mut watch = Watch::new("S10.full", harness);
            watch.scenario = Some("S10.full");
            let wait = RashidWait::new(harness, Duration::from_secs(12));
            let started = tokio::time::Instant::now();
            let result = watch
                .run(
                    None,
                    "indexed row",
                    wait.run(async {
                        harness.bounded_wait(started + Duration::from_secs(12), "rashid-rotation");
                        look_for_rashid(
                            &mut (),
                            (),
                            &wait,
                            async |_, _| std::future::pending().await,
                            async |_, _| Err("unexpected LOOK backoff".into()),
                        )
                        .await
                    }),
                )
                .await;
            let error = result.err().ok_or("stalled LOOK should fail")?;
            assert_eq!(started.elapsed(), Duration::from_secs(1));
            assert!(matches!(
                classify(error.as_ref()),
                Cause::NoProgress {
                    phase: "rashid-look",
                    ..
                }
            ));
            Ok(())
        })
    }

    #[test]
    fn packet_observation_extends_only_the_existing_timer() -> TestResult {
        runtime()?.block_on(async {
            let harness = Box::leak(Box::new(Harness::new(true, Some(Duration::from_secs(90)))));
            let watch = Watch::new("P10", harness);
            let start = tokio::time::Instant::now();
            watch
                .bound(Duration::from_secs(200), "wall budget", async {
                    harness.bounded_wait(start + Duration::from_secs(180), "resting stamina");
                    tokio::time::sleep(Duration::from_secs(180)).await;
                    Ok(())
                })
                .await?;
            assert_eq!(start.elapsed(), Duration::from_secs(180));
            Ok(())
        })
    }

    #[test]
    fn qualification_spawn_wait_renews_watchdog_exemption_after_walk() -> TestResult {
        runtime()?.block_on(async {
            let harness = Box::leak(Box::new(Harness::new(true, Some(Duration::from_secs(10)))));
            let watch = new_watch(harness);
            let deadline = tokio::time::Instant::now() + Duration::from_secs(95);
            watch
                .bound(Duration::from_secs(100), "row deadline", async {
                    harness.bounded_wait(deadline, "spawn observation");
                    harness.progress("walk");
                    crate::qualification::client_world::wait_for_spawn_observation(
                        harness,
                        deadline,
                        async {
                            tokio::time::sleep(Duration::from_secs(90)).await;
                            Ok(())
                        },
                    )
                    .await
                })
                .await
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
                    Err(harness.fixture_failed(
                        Need::Fixture([0; 16], [0; 16]),
                        0,
                        &std::io::Error::other("release expired"),
                    ))
                })
                .await;
            assert_eq!(started.elapsed(), Duration::from_secs(162));
            assert!(matches!(
                error.as_ref().err().map(|error| classify(error.as_ref())),
                Some(Cause::Precondition { .. })
            ));
            assert!(
                harness
                    .fixture_gate(Need::Fixture([0; 16], [0; 16]))
                    .is_err()
            );
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
mod advisory_tests {
    #[test]
    fn calibration_verdict_is_not_a_scored_species() {
        let line =
            "e2e_all scenario=CS.calibration.cyclops.attempt1.full status=FAIL seconds=0.000"
                .to_owned();
        assert!(!super::advisory_line(line.clone(), true).starts_with("e2e_all "));
        assert_eq!(super::advisory_line(line.clone(), false), line);
    }
}

#[cfg(test)]
mod capture_tests {
    use super::*;

    #[test]
    fn indexed_capture_completion_is_a_terminal_pass() {
        let harness = Box::leak(Box::new(Harness::new(true, None)));
        let mut watch = Watch::new("C02.client", harness);
        watch.scenario = Some("C02.client");
        let active = watch.enter();

        assert_eq!(
            indexed_evidence_for(
                harness,
                "e2e_capture scenario=C02.client status=CAPTURED attack=1".to_owned()
            ),
            None
        );
        drop(active);
        assert_eq!(
            watch.take_verdicts(),
            ["e2e_all scenario=C02.client status=PASS seconds=0.000 evidence=attack=1"]
        );
    }

    #[test]
    fn capture_diagnostic_prevents_terminal_pass() {
        let harness = Box::leak(Box::new(Harness::new(true, None)));
        let mut watch = Watch::new("E01.client", harness);
        watch.scenario = Some("E01.client");
        let active = watch.enter();
        assert!(
            indexed_evidence_for(
                harness,
                "E01_CLIENT_CAPTURE_WRITE_ERROR file=look error=denied".into()
            )
            .is_none()
        );
        assert!(
            indexed_evidence_for(
                harness,
                "e2e_capture scenario=E01.client status=CAPTURED".into()
            )
            .is_none()
        );
        drop(active);
        let verdicts = watch.take_verdicts();
        assert_eq!(verdicts.len(), 1);
        assert!(verdicts[0].contains("status=FAIL"));
    }

    #[test]
    fn capture_write_failure_has_one_terminal_verdict() {
        let harness = Box::leak(Box::new(Harness::new(true, None)));
        let mut watch = Watch::new("I03.client", harness);
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
