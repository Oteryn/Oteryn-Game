// NODEBOOT-1: a step that found the single holder busy retries when the holder is returned.

/// Run one semantic pass that keeps the holder for `hold`.
async fn hold_holder(root: &super::DurabilityRoot, hold: std::time::Duration) -> Result<(), crate::durability::DurabilityError> {
    root.try_issue_semantic_pass()?
        .run(move |_holder, _deadline| {
            Box::pin(async move {
                tokio::time::sleep(hold).await;
                Ok(())
            })
        })
        .await
}

async fn ready_root(database_url: &str) -> Result<super::DurabilityRoot, Box<dyn Error>> {
    let root = super::DurabilityRoot::connect_test_runtime(database_url)?;
    if !root.maintain_ready_once().await? {
        return Err("the holder was not established".into());
    }
    Ok(root)
}

#[test]
fn foreground_admission_recovers_a_holder_retired_after_a_successful_pass() -> TestResult {
    run_postgres_test(async {
        let (database, url) = migrated_database("holder_foreground_retired").await?;
        let root = ready_root(&url).await?;
        root.try_issue_semantic_pass()?.run(|_holder, _| Box::pin(async { Ok(()) })).await?;
        root.try_acquire_ready()?.close().await?;
        assert!(!root.is_ready());
        assert!(matches!(root.try_issue_semantic_pass(),
            Err(crate::durability::DurabilityError::RootUnavailable)));
        let value: i32 = root.issue_ready_semantic_pass().await?.run(|holder, _| {
            Box::pin(async move { Ok(sqlx::query_scalar("SELECT 1").fetch_one(&mut **holder).await?) })
        }).await?;
        assert_eq!(value, 1);
        database.cleanup().await
    })
}

#[test]
fn foreground_admission_waits_for_the_busy_holder() -> TestResult {
    run_postgres_test(async {
        let (database, url) = migrated_database("holder_foreground_busy").await?;
        let root = ready_root(&url).await?;
        let holder = root.try_acquire_ready()?;
        let started = std::time::Instant::now();
        let release = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            drop(holder);
        });
        let pass = root.issue_ready_semantic_pass().await?;
        assert!(started.elapsed() >= std::time::Duration::from_millis(100));
        let value: i32 = pass.run(|holder, _| Box::pin(async move {
            Ok(sqlx::query_scalar("SELECT 1").fetch_one(&mut **holder).await?)
        })).await?;
        assert_eq!(value, 1);
        release.await?;
        database.cleanup().await
    })
}

#[test]
fn foreground_admission_is_bounded_and_preserves_recovery_demand() -> TestResult {
    run_postgres_test(async {
        let (database, url) = migrated_database("holder_foreground_budget").await?;
        let root = ready_root(&url).await?;
        let holder = root.try_acquire_ready()?;
        let started = std::time::Instant::now();
        let result = tokio::time::timeout(
            super::ROOT_RECOVERY_WINDOW + std::time::Duration::from_secs(1),
            root.issue_ready_semantic_pass(),
        ).await?;
        let error = result.err().ok_or("busy root was admitted past its budget")?;
        assert!(matches!(error, crate::durability::DurabilityError::RootUnavailable), "{error:?}");
        assert!(started.elapsed() >= super::ROOT_RECOVERY_WINDOW);
        assert!(root.has_ready_demand());
        drop(holder);
        root.await_ready().await?;
        database.cleanup().await
    })
}

#[test]
fn a_busy_holder_retry_starts_when_the_holder_is_returned() -> TestResult {
    run_postgres_test(async {
        let (database, database_url) = migrated_database("holder_retry_returned").await?;
        let root = ready_root(&database_url).await?;
        let pass = tokio::spawn({
            let root = root.clone();
            async move { hold_holder(&root, std::time::Duration::from_millis(300)).await }
        });
        tokio::task::yield_now().await;
        assert!(matches!(
            root.try_acquire_ready(),
            Err(crate::durability::DurabilityError::RootUnavailable)
        ));
        let started = tokio::time::Instant::now();
        root.wait_holder_retry(
            started + std::time::Duration::from_millis(50),
            started + std::time::Duration::from_secs(5),
        )
        .await;
        let waited = started.elapsed();
        assert!(waited < std::time::Duration::from_secs(2), "{waited:?}");
        assert!(root.try_acquire_ready().is_ok());
        pass.await??;
        database.cleanup().await
    })
}

#[test]
fn an_idle_holder_retry_waits_the_whole_backoff() -> TestResult {
    run_postgres_test(async {
        let (database, database_url) = migrated_database("holder_retry_idle").await?;
        let root = ready_root(&database_url).await?;
        let started = tokio::time::Instant::now();
        let latest = started + std::time::Duration::from_millis(200);
        root.wait_holder_retry(started + std::time::Duration::from_millis(50), latest)
            .await;
        assert!(tokio::time::Instant::now() >= latest);
        assert!(root.is_ready());
        database.cleanup().await
    })
}

#[test]
fn a_holder_retry_never_establishes_the_holder() -> TestResult {
    run_postgres_test(async {
        let (database, database_url) = migrated_database("holder_retry_absent").await?;
        let root = super::DurabilityRoot::connect_test_runtime(&database_url)?;
        let started = tokio::time::Instant::now();
        let latest = started + std::time::Duration::from_millis(200);
        root.wait_holder_retry(started + std::time::Duration::from_millis(50), latest)
            .await;
        assert!(tokio::time::Instant::now() >= latest);
        assert_eq!(root.pool.size(), 0);
        assert!(matches!(
            root.try_acquire_ready(),
            Err(crate::durability::DurabilityError::RootUnavailable)
        ));
        database.cleanup().await
    })
}

/// A scheduled owner retries immediately after returning the holder, without the
/// old artificial 1 ms gap. A foreground demand remains visible until checkout.
#[test]
fn a_busy_holder_retry_wins_against_background_passes() -> TestResult {
    run_postgres_test(async {
        let (database, database_url) = migrated_database("holder_retry_contended").await?;
        let root = ready_root(&database_url).await?;
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let first_background = root.try_issue_background_semantic_pass()?;
        let background = tokio::spawn({
            let root = root.clone();
            let stop = std::sync::Arc::clone(&stop);
            async move {
                first_background
                    .run(|_, _| Box::pin(async { Ok(()) }))
                    .await?;
                while !stop.load(Ordering::Acquire) {
                    match root.try_issue_background_semantic_pass() {
                        Ok(pass) => {
                            pass.run(|_, _| Box::pin(async { Ok(()) })).await?;
                        }
                        Err(crate::durability::DurabilityError::RootUnavailable) => {
                            tokio::task::yield_now().await;
                        }
                        Err(error) => return Err(error),
                    }
                }
                Ok::<(), crate::durability::DurabilityError>(())
            }
        });
        for _ in 0..10 {
            let _foreground = root.foreground_demand()?;
            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                match hold_holder(&root, std::time::Duration::from_millis(1)).await {
                    Ok(()) => break,
                    Err(crate::durability::DurabilityError::RootUnavailable) => {
                        let now = tokio::time::Instant::now();
                        assert!(now < deadline, "the step never got the holder");
                        root.wait_holder_retry(
                            now + std::time::Duration::from_millis(50),
                            (now + std::time::Duration::from_millis(200)).min(deadline),
                        )
                        .await;
                    }
                    Err(error) => return Err(error.into()),
                }
            }
        }
        stop.store(true, Ordering::Release);
        background.await??;
        database.cleanup().await
    })
}

#[test]
fn foreground_demand_survives_holder_return_without_background_gap() -> TestResult {
    run_postgres_test(async {
        let (database, url) = migrated_database("holder_foreground_no_gap").await?;
        let root = ready_root(&url).await?;
        let background = root.clone();
        let held = background.try_issue_background_semantic_pass()?;
        let foreground = root.foreground_demand()?;
        let started = tokio::time::Instant::now();
        let waiting = root.wait_holder_retry(started, started + super::ROOT_RECOVERY_WINDOW);
        tokio::pin!(waiting);
        let polled = std::future::poll_fn(|cx| {
            std::task::Poll::Ready(std::future::Future::poll(waiting.as_mut(), cx))
        }).await;
        assert!(polled.is_pending());
        held.run(|_, _| Box::pin(async { Ok(()) })).await?;
        // No yield/sleep after return: reproduce the overdue scheduler's immediate
        // retake while the foreground owner has not been polled again yet.
        for _ in 0..16 {
            assert!(matches!(
                background.try_issue_background_semantic_pass(),
                Err(crate::durability::DurabilityError::RootUnavailable)
            ));
        }
        waiting.await;
        let value: i32 = root.try_issue_semantic_pass()?.run(|holder, _| {
            Box::pin(async move {
                Ok(sqlx::query_scalar("SELECT 1").fetch_one(&mut **holder).await?)
            })
        }).await?;
        assert_eq!(value, 1);
        drop(foreground);
        background
            .try_issue_background_semantic_pass()?
            .run(|_, _| Box::pin(async { Ok(()) }))
            .await?;
        assert_eq!(root.pool.size(), 1);
        database.cleanup().await
    })
}

#[test]
fn background_fairness_window_serves_idle_holder_despite_a_waiting_ticket() -> TestResult {
    run_postgres_test(async {
        let (database, url) = migrated_database("holder_background_waiting_ticket").await?;
        let root = ready_root(&url).await?;
        let foreground = root.foreground_demand()?;
        let until = std::time::Instant::now() + super::ROOT_RECOVERY_WINDOW;
        // A different retry owner keeps its ticket while the scheduler asks for a pass.
        let ticket = tokio::spawn({
            let root = root.clone();
            async move { root.holder_waiter() }
        });
        let owner = super::holder_queue::OwnerKey::Task(ticket.id());
        let ticket = ticket.await?.ok_or("waiting owner's ticket")?;
        assert!(root.pool.num_idle() > 0);
        assert!(matches!(
            root.try_issue_semantic_pass(),
            Err(crate::durability::DurabilityError::RootUnavailable)
        ));
        assert!(
            matches!(
                root.try_issue_background_semantic_pass(),
                Err(crate::durability::DurabilityError::RootUnavailable)
            ),
            "tickets keep priority before the fairness window expires"
        );
        loop {
            let now = std::time::Instant::now();
            root.holder_queue
                .lock()
                .map_err(|_| "holder queue poisoned")?
                .refused(owner, now);
            if now >= until {
                break;
            }
            // Keep the ticket live through the window, as sustained retries do.
            tokio::time::sleep_until(tokio::time::Instant::from_std(
                (now + std::time::Duration::from_millis(100)).min(until),
            ))
            .await;
        }
        assert!(matches!(
            root.try_issue_semantic_pass(),
            Err(crate::durability::DurabilityError::RootUnavailable)
        ));
        let pass = root.try_issue_background_semantic_pass();
        assert!(
            pass.is_ok(),
            "expired fairness window must serve background despite a waiting ticket"
        );
        let value: i32 = pass?
            .run(|holder, _| {
                Box::pin(async move {
                    Ok(sqlx::query_scalar("SELECT 1")
                        .fetch_one(&mut **holder)
                        .await?)
                })
            })
            .await?;
        assert_eq!(value, 1);
        assert!(
            matches!(
                root.try_issue_background_semantic_pass(),
                Err(crate::durability::DurabilityError::RootUnavailable)
            ),
            "one background pass renews foreground preference"
        );
        assert!(
            matches!(
                root.try_issue_semantic_pass(),
                Err(crate::durability::DurabilityError::RootUnavailable)
            ),
            "background service must preserve the waiting ticket"
        );
        drop(ticket);
        drop(foreground);
        database.cleanup().await
    })
}

// NBHOLD-1: scheduled work also uses ordinary passes (journal, Character, etc.).
#[test]
fn nbhold_scheduled_ordinary_passes_yield_to_foreground_within_command_window() -> TestResult {
    run_postgres_test(async {
        let (database, url) = migrated_database("nbhold_ordinary_background").await?;
        let root = ready_root(&url).await?;
        let held = root.try_issue_semantic_pass()?;
        let foreground = root.foreground_demand()?;
        let ticket = root.holder_waiter();
        let (returned, release) = tokio::sync::oneshot::channel();
        let background = root.spawn_task({
            let root = root.clone();
            async move {
                super::DurabilityRoot::with_background_holder(async {
                    held.run(|_, _| Box::pin(async { Ok(()) })).await?;
                    // Reproduce a background continuation polled first on every return.
                    // Its journal reads spawn onto the root runtime as well.
                    for _ in 0..16 {
                        let root_in = root.clone();
                        let acquired = root.spawn_task(async move {
                            match root_in.try_issue_semantic_pass() {
                                Ok(pass) => {
                                    pass.run(|_, _| Box::pin(async { Ok(()) })).await?;
                                    Ok(true)
                                }
                                Err(crate::durability::DurabilityError::RootUnavailable) => Ok(false),
                                Err(error) => Err(error),
                            }
                        }).await.map_err(|_| crate::durability::DurabilityError::RootTaskFailed)??;
                        assert!(!acquired, "scheduled ordinary pass overtook queued foreground");
                    }
                    let _ = returned.send(());
                    Ok::<(), crate::durability::DurabilityError>(())
                }).await
            }
        });
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(1);
        // Observe that background yielded before allowing the foreground to run.
        let released = tokio::time::timeout_at(deadline, release).await?;
        background.await??;
        released?;
        root.wait_holder_turn(tokio::time::Instant::now(), deadline).await;
        let value: i32 = tokio::time::timeout_at(deadline,
            root.try_issue_semantic_pass()?.run(|holder, _| Box::pin(async move {
                Ok(sqlx::query_scalar("SELECT 1").fetch_one(&mut **holder).await?)
            }))
        ).await??;
        assert_eq!(value, 1);
        drop(ticket);
        drop(foreground);
        super::DurabilityRoot::with_background_holder(async {
            root.try_issue_semantic_pass()?.run(|_, _| Box::pin(async { Ok(()) })).await
        }).await?;
        database.cleanup().await
    })
}
