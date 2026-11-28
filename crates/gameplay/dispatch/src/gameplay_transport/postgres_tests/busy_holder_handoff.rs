use super::TestResult;
use super::fixture::Harness;
use crate::durability::{DurabilityRoot, ROOT_RECOVERY_WINDOW};
use sqlx::pool::PoolConnectionReturnDisposition;
use std::future::{Future, poll_fn};
use std::task::Poll;
use std::time::{Duration, Instant};

fn run(
    admin: String,
    name: &'static str,
    exercise: impl AsyncFnOnce(&Harness) -> TestResult + Send + 'static,
) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let runtime =
                crate::bestiary_postgres_harness::runtime().map_err(|error| error.to_string())?;
            runtime
                .block_on(async move {
                    let h = Harness::create(admin, name, false).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result
                })
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "busy-holder PG thread failed")??;
    Ok(())
}

#[test]
fn busy_holder_cancelled_head_hands_off_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "busy_holder_cancelled_head", async |h| {
        let root = &h.root;
        let mut held = root.try_acquire_ready()?;
        let oldest = DurabilityRoot::with_holder_owner(async {
            let _ticket = root.holder_waiter().ok_or("oldest ticket")?;
            let now = tokio::time::Instant::now();
            root.wait_holder_turn(now, now + ROOT_RECOVERY_WINDOW).await;
            std::future::pending::<TestResult>().await
        });
        let mut oldest = Box::pin(oldest);
        poll_fn(|cx| {
            assert!(oldest.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        let wait = || DurabilityRoot::with_holder_owner(root.issue_ready_semantic_pass());
        let mut second = Box::pin(wait());
        let mut third = Box::pin(wait());
        poll_fn(|cx| {
            assert!(second.as_mut().poll(cx).is_pending());
            assert!(third.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        assert_eq!(
            held.return_to_pool_observed_until(Instant::now() + ROOT_RECOVERY_WINDOW)
                .await,
            PoolConnectionReturnDisposition::ReturnedToIdle,
        );
        assert!(
            root.try_acquire_ready().is_err(),
            "the idle holder is reserved for the head"
        );
        drop(oldest);
        let second = tokio::time::timeout(Duration::from_secs(1), second).await??;
        assert!(!root.is_ready());
        second.run(|_, _| Box::pin(async { Ok(()) })).await?;
        tokio::time::timeout(Duration::from_secs(1), third)
            .await??
            .run(|_, _| Box::pin(async { Ok(()) }))
            .await?;
        Ok(())
    })
}

#[test]
fn busy_holder_background_grant_progress_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "busy_holder_background_grant", async |h| {
        let root = &h.root;
        let (first_pass, ready) = tokio::sync::oneshot::channel();
        let (queued, tickets) = tokio::sync::oneshot::channel();
        let (complete, completed) = tokio::sync::oneshot::channel();
        let background =
            DurabilityRoot::with_background_holder(root.with_background_operation(async {
                root.try_issue_semantic_pass()?
                    .run(|_, _| Box::pin(async { Ok(()) }))
                    .await?;
                first_pass.send(()).map_err(|_| "first pass")?;
                tickets.await?;
                assert!(
                    root.background_holder_ready(),
                    "the granted unit can pass queued foreground"
                );
                for _ in 0..2 {
                    root.try_issue_semantic_pass()?
                        .run(|holder, _| {
                            Box::pin(async {
                                assert_eq!(
                                    sqlx::query_scalar::<_, i32>("SELECT 1")
                                        .fetch_one(&mut **holder)
                                        .await?,
                                    1
                                );
                                Ok(())
                            })
                        })
                        .await?;
                }
                complete.send(()).map_err(|_| "completion signal")?;
                Ok::<(), Box<dyn std::error::Error>>(())
            }));
        let foreground = DurabilityRoot::with_holder_owner(async {
            ready.await?;
            let _demand = root.foreground_demand()?;
            let _ticket = root.holder_waiter().ok_or("foreground ticket")?;
            queued.send(()).map_err(|_| "foreground readiness")?;
            // Keep the reserved turn while the granted operation finishes.
            completed.await?;
            root.try_issue_semantic_pass()?
                .run(|_, _| Box::pin(async { Ok(()) }))
                .await?;
            Ok::<_, Box<dyn std::error::Error>>(())
        });
        let (background, foreground) = tokio::time::timeout(Duration::from_secs(2), async {
            tokio::join!(background, foreground)
        })
        .await?;
        background?;
        foreground?;
        Ok(())
    })
}

#[test]
fn busy_holder_timeout_names_raw_checkout_owner_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let runtime =
                crate::bestiary_postgres_harness::runtime().map_err(|error| error.to_string())?;
            let h = runtime
                .block_on(Harness::create(admin, "busy_holder_timeout_owner", false))
                .map_err(|error| error.to_string())?;
            let mut result = Ok(());
            let lines = oteryn_error_codes::capture(|| {
                result = runtime
                    .block_on(async {
                        let mut held = h.root.try_acquire_ready()?;
                        let description =
                            h.root.holder_owner_description().ok_or("missing owner")?;
                        assert!(
                            description.contains("busy_holder_handoff.rs"),
                            "{description}"
                        );
                        h.root.request_ready();
                        assert!(h.root.maintain_ready_once().await.is_err());
                        assert!(h.root.holder_owner_description().is_some());
                        assert_eq!(
                            held.return_to_pool_observed_until(
                                Instant::now() + ROOT_RECOVERY_WINDOW
                            )
                            .await,
                            PoolConnectionReturnDisposition::ReturnedToIdle,
                        );
                        Ok::<_, Box<dyn std::error::Error>>(())
                    })
                    .map_err(|error| error.to_string());
            });
            runtime
                .block_on(h.cleanup())
                .map_err(|error| error.to_string())?;
            result?;
            let timeout = lines
                .iter()
                .find(|line| line.contains("event=holder_stuck"))
                .ok_or("holder timeout did not log")?;
            assert!(
                timeout.contains("busy_holder_handoff.rs") && timeout.contains("held_ms="),
                "{timeout}"
            );
            Ok::<_, String>(())
        })?
        .join()
        .map_err(|_| "busy-holder log PG thread failed")??;
    Ok(())
}
