//! Node boot must survive the Soulpit owner cadence before any player enters.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult};

#[test]
fn soulpit_empty_world_runtime_role_boot_cadence_postgres() -> TestResult {
    scenario(false)
}

#[test]
fn soulpit_boot_read_contention_does_not_retire_channel_postgres() -> TestResult {
    scenario(true)
}

fn scenario(contended: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        eprintln!("SOULPIT_BOOT_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("soulpit-boot", move || {
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
                {
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
    })
}
