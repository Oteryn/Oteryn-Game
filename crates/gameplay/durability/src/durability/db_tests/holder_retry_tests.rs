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

/// Background passes keep the holder about 99% busy, as the boot-time highscore, guild and
/// market passes did. A step retrying on the holder's return still gets it within its window.
#[test]
fn a_busy_holder_retry_wins_against_background_passes() -> TestResult {
    run_postgres_test(async {
        let (database, database_url) = migrated_database("holder_retry_contended").await?;
        let root = ready_root(&database_url).await?;
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let background = tokio::spawn({
            let root = root.clone();
            let stop = std::sync::Arc::clone(&stop);
            async move {
                while !stop.load(Ordering::Acquire) {
                    let _ = hold_holder(&root, std::time::Duration::from_millis(100)).await;
                    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                }
            }
        });
        for _ in 0..10 {
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
        background.await?;
        database.cleanup().await
    })
}
