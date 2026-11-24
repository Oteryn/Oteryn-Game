//! Node boot must survive the Soulpit owner cadence before any player enters.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult};

#[test]
fn soulpit_empty_world_runtime_role_boot_cadence_postgres() -> TestResult {
    scenario(Case::Empty)
}

#[test]
fn soulpit_boot_read_contention_does_not_retire_channel_postgres() -> TestResult {
    scenario(Case::Locked)
}

#[test]
fn soulpit_boot_retries_discovery_on_the_real_holder_return() -> TestResult {
    scenario(Case::HolderReturn)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Case {
    Empty,
    Locked,
    HolderReturn,
}

fn scenario(case: Case) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        eprintln!("SOULPIT_BOOT_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("soulpit-boot", move || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "soulpit_boot", true, false).await?;
            {
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let deps = Dependencies::new()?;
                let admission = deps.owner(&h, &authority);
                h.root
                    .try_issue_semantic_pass()
                    .map_err(debug)?
                    .run(|holder, _| {
                        Box::pin(async move {
                            sqlx::query("SET ROLE oteryn_game_runtime")
                                .execute(&mut **holder)
                                .await?;
                            Ok(())
                        })
                    })
                    .await
                    .map_err(debug)?;
                let owner =
                    crate::durability::activity_handoff::ActivityHandoffOwner::new(h.root.clone());
                let bindings = owner
                    .active_soulpits(&authority, &h.node, admission.world_id, None)
                    .await
                    .map_err(debug)?;
                assert!(bindings.is_empty());
                if case == Case::HolderReturn {
                    use std::future::{Future, poll_fn};
                    use std::task::Poll;

                    // Keep the retried discovery in PostgreSQL until its checkout
                    // is observable even when a poll can finish a fast read.
                    let mut discovery_gate = h.pool.begin().await?;
                    sqlx::query("LOCK TABLE game_durability_admission_account_guards IN ACCESS EXCLUSIVE MODE")
                        .execute(&mut *discovery_gate)
                        .await?;
                    let held = h.root.try_issue_semantic_pass()?;
                    assert!(!h.root.background_holder_ready());
                    let _owner_clock = admission.freeze_owner_clock_for_test();
                    let stop = oteryn_foundation::CancellationToken::new();
                    let mut cadence = std::pin::pin!(admission.run_soulpits(&stop));
                    tokio::time::pause();
                    let paused_at = tokio::time::Instant::now();
                    poll_fn(|cx| {
                        assert!(cadence.as_mut().poll(cx).is_pending());
                        Poll::Ready(())
                    })
                    .await;
                    tokio::select! {
                        result = held.run(|_, _| Box::pin(async { Ok(()) })) => result?,
                        // Keep the clock at the same instant while PostgreSQL services
                        // the holder return. Only its notification can wake discovery.
                        () = async {
                            let started = std::time::Instant::now();
                            while started.elapsed() < std::time::Duration::from_secs(1) {
                                tokio::task::yield_now().await;
                            }
                        } => return Err("PostgreSQL did not return the test holder".into()),
                    }
                    assert!(h.root.background_holder_ready());
                    // Keep driving the cadence until checkout. A single poll
                    // can stop at a pending operation before it takes the holder.
                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
                    let checked_out = poll_fn(|cx| {
                        assert!(cadence.as_mut().poll(cx).is_pending());
                        if !h.root.is_ready() {
                            return Poll::Ready(true);
                        }
                        if std::time::Instant::now() >= deadline {
                            return Poll::Ready(false);
                        }
                        cx.waker().wake_by_ref();
                        Poll::Pending
                    })
                    .await;
                    assert_eq!(tokio::time::Instant::now(), paused_at);
                    assert!(
                        checked_out,
                        "the next discovery has checked out the returned holder"
                    );
                    tokio::time::resume();
                    stop.cancel();
                    assert_eq!(
                        cadence.await,
                        crate::monster_owner_cycle::respawn_job::RespawnExit::Shutdown
                    );
                    discovery_gate.rollback().await?;
                } else {
                    let contended = case == Case::Locked;
                    let mut blocker = if contended {
                        let mut tx = h.pool.begin().await?;
                        sqlx::query("LOCK TABLE game_durability_admission_account_guards IN ACCESS EXCLUSIVE MODE").execute(&mut *tx).await?;
                        Some(tx)
                    } else {
                        None
                    };
                    let stop = oteryn_foundation::CancellationToken::new();
                    let mut cadence = std::pin::pin!(admission.run_soulpits(&stop));
                    assert!(
                        tokio::time::timeout(
                            std::time::Duration::from_millis(if contended { 2_500 } else { 100 }),
                            &mut cadence
                        )
                        .await
                        .is_err(),
                        "empty-world Soulpit cadence must remain live"
                    );
                    if let Some(tx) = blocker.take() {
                        tx.rollback().await?;
                    }
                    stop.cancel();
                    assert_eq!(
                        cadence.await,
                        crate::monster_owner_cycle::respawn_job::RespawnExit::Shutdown
                    );
                }
                assert!(deps.runtime.lock().await.owner_fence().is_ok());
                h.root.await_ready().await.map_err(debug)?;
                assert!(
                    owner
                        .active_soulpits(&authority, &h.node, admission.world_id, None)
                        .await
                        .map_err(debug)?
                        .is_empty()
                );
            }
            h.cleanup().await
        })
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED soulpit_boot {case:?} elapsed={:?}",
        started.elapsed()
    );
    Ok(())
}
