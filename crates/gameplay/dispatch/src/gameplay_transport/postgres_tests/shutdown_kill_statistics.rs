use super::*;
use crate::durability::public_projections::daily_kills::KILL_BATCH_MAX;
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::{LastPositionFlush, serving::QuiescedKillOwners};

#[test]
fn shutdown_checkpoint_flushes_intermediate_kill_statistics_batches() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    kill_reward_e2e_fixtures::scenario_thread::run_async(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "shutdown_kill_statistics", false, false)
                .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        {
            let mut runtime = owner.runtime.lock().await;
            let attack = owner.states.attack.lock().await;
            let now = crate::gameplay_transport::npc_trade_command::unix_ms_now();
            for _ in 0..=KILL_BATCH_MAX {
                let actor = runtime
                    .admit_pinned_test_creature(MovementLocalPosition {
                        x: 2,
                        y: 2,
                        floor: 7,
                    })
                    .map_err(debug)?;
                runtime
                    .commit_monster_lab_damage(actor, b"shutdown-statistics", 20)
                    .map_err(debug)?;
                let mut death = runtime.borrow_combat_death();
                let receipt = death.committed_lethal_receipt(actor).map_err(debug)?;
                let key = receipt.statistics_death_key().map_err(debug)?;
                death.project_committed_lethal(receipt).map_err(debug)?;
                attack.kills().statistics().capture(
                    &runtime,
                    actor,
                    key,
                    "oteryn:creature.rat",
                    false,
                    now,
                );
                runtime.remove_test_actor(actor).map_err(debug)?;
            }
        }
        let flush = LastPositionFlush::default();
        flush.request();
        assert!(flush.serve(&owner, &QuiescedKillOwners::for_test()).await);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_statistics_creature_deaths")
                .fetch_one(&h.pool)
                .await?,
            i64::try_from(KILL_BATCH_MAX + 1)?
        );
        let clean: bool =
            sqlx::query_scalar("SELECT clean FROM game_statistics_kill_sources WHERE generation=1")
                .fetch_one(&h.pool)
                .await?;
        assert!(clean);
        eprintln!("MULTIBATCH_SHUTDOWN_KILLS_POSTGRES_CASE_COMPLETED");
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
