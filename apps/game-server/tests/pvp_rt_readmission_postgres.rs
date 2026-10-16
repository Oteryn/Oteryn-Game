#![cfg(any())] // CP slot: enable the routed native readmission target.
#![allow(clippy::expect_used)]
// Ordinary native Root composition, following combat_bestiary_postgres.rs.
// The actual PVP-1 fixture is read-only; no copied guards or helper widening.
extern crate oteryn_game_server as production_server;
extern crate self as oteryn_game_server;
pub use production_server::admission_evidence;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_bootstrap_intent.rs"]
pub mod character_bootstrap_intent;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_recovery_fence.rs"]
pub mod character_recovery_fence;
pub use production_server::domain;
#[allow(dead_code)]
#[path = "support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;
#[allow(dead_code, unused_imports)]
#[path = "../src/combat.rs"]
pub mod combat;
#[allow(dead_code, unused_imports)]
#[path = "../src/durability/mod.rs"]
mod durability;
#[allow(dead_code, unused_imports)]
#[path = "../src/foundation/mod.rs"]
pub mod foundation;
#[allow(dead_code, unused_imports)]
#[path = "../src/native_admission_source/mod.rs"]
pub mod native_admission_source;
#[allow(dead_code)]
#[path = "../src/combat/pvp/lock.rs"]
mod pvp_readmission_lock;

mod readmission_cases {
    use crate::bestiary_postgres_harness::{
        CHANNEL, CHARACTER, Harness, TestResult, WORLD, configured_admin, debug, fence, id, runtime,
    };
    use crate::durability::DurabilityRoot;
    use crate::durability::character_progression::CurrentCharacterGameplayFence;
    use crate::foundation::owner_timer::{SemanticTimeMicros as OwnerTime, VirtualOwnerClock};
    use crate::foundation::{
        ChannelId, ConnectionGeneration, GameSessionId, RuntimeScopeRefV1,
        ScopeOwnershipGeneration, WorldId,
    };
    use crate::pvp_readmission_lock::{CombatLock, PvpReadmissionError};
    use oteryn_simulation_determinism::SemanticTimeMicros as SimTime;

    fn run<F>(tag: &'static str, body: F) -> TestResult
    where
        F: AsyncFnOnce(&Harness) -> TestResult,
    {
        let admin = configured_admin().ok_or("Canonical PostgreSQL configuration is mandatory")?;
        runtime()?.block_on(async move {
            let h = Harness::create(admin, tag, true).await?;
            let version: String = sqlx::query_scalar("SHOW server_version_num")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(version, "170006");
            eprintln!("PVP-READMISSION-NATIVE-PG fixture={tag} server_version_num={version}");
            let result = body(&h).await;
            h.cleanup().await?;
            result
        })
    }

    // Administrative fixture consequence DATA, not a production writer or authority producer.
    // The unchanged original Harness supplies real recovery/Root/node/assignment/session APIs.
    async fn stored(
        h: &Harness,
        logout: Option<i64>,
        pz: Option<i64>,
        kill: Option<i64>,
    ) -> TestResult {
        sqlx::query(
            "INSERT INTO game_character_pvp_state \
            (character_id,world_id,logout_block_until,pz_block_until,kill_block_until) \
            VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid, \
            clock_timestamp()+$3*interval '1 millisecond', \
            clock_timestamp()+$4*interval '1 millisecond', \
            clock_timestamp()+$5*interval '1 millisecond')",
        )
        .bind(id(CHARACTER).as_slice())
        .bind(id(WORLD).as_slice())
        .bind(logout)
        .bind(pz)
        .bind(kill)
        .execute(&h.pool)
        .await?;
        Ok(())
    }

    async fn call(
        h: &Harness,
        lock: &mut CombatLock,
        clock: &VirtualOwnerClock,
        expected: CurrentCharacterGameplayFence,
    ) -> TestResult<Result<bool, PvpReadmissionError>> {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        Ok(lock
            .restore_pvp_readmission(&h.root, &authority, &h.node, expected, clock)
            .await)
    }

    fn prior_lock() -> CombatLock {
        let mut lock = CombatLock::default();
        let future = SimTime::from_micros(1_000_000_000_000);
        lock.on_pvp_hit(future);
        lock.on_aggression(future);
        lock.on_kill(future);
        lock
    }

    #[test]
    fn readmission_restores_each_real_stored_deadline_on_owner_time() -> TestResult {
        for (tag, logout, pz, kill) in [
            ("pvp_readmit_logout", Some(120_000), None, None),
            ("pvp_readmit_pz", None, Some(120_000), None),
            ("pvp_readmit_kill", None, None, Some(120_000)),
        ] {
            run(tag, async |h| {
                stored(h, logout, pz, kill).await?;
                let clock = VirtualOwnerClock::new(OwnerTime::from_micros(5_000_000));
                let mut lock = CombatLock::default();
                assert!(
                    call(h, &mut lock, &clock, fence(1)?)
                        .await?
                        .map_err(debug)?
                );
                let live = SimTime::from_micros(6_000_000);
                assert!(lock.blocks_logout(live, None));
                assert_eq!(lock.pz_block_running(live), pz.is_some());
                assert_eq!(lock.kill_block_running(live), kill.is_some());
                assert_eq!(lock.blocks_pz_entry(live), pz.is_some() || kill.is_some());
                assert!(!lock.blocks_logout(SimTime::from_micros(126_000_000), None));
                assert_eq!(h.root_revision().await?, "1");
                Ok(())
            })?;
        }
        Ok(())
    }

    #[test]
    fn readmission_no_row_and_expired_row_never_clear_prior_locks() -> TestResult {
        run("pvp_readmit_absent_expired", async |h| {
            let clock = VirtualOwnerClock::new(OwnerTime::from_micros(1_000));
            let mut lock = prior_lock();
            let before = lock;
            assert!(
                !call(h, &mut lock, &clock, fence(1)?)
                    .await?
                    .map_err(debug)?
            );
            assert_eq!(lock, before);
            stored(h, Some(-1), Some(-1), Some(-1)).await?;
            assert!(
                call(h, &mut lock, &clock, fence(1)?)
                    .await?
                    .map_err(debug)?
            );
            assert_eq!(lock, before);
            Ok(())
        })
    }

    #[test]
    fn readmission_repeated_shorter_data_cannot_shorten_existing_lock() -> TestResult {
        run("pvp_readmit_no_shorten", async |h| {
            stored(h, Some(60_000), Some(60_000), Some(60_000)).await?;
            let clock = VirtualOwnerClock::new(OwnerTime::from_micros(1_000));
            let mut lock = prior_lock();
            let before = lock;
            for _ in 0..2 {
                assert!(
                    call(h, &mut lock, &clock, fence(1)?)
                        .await?
                        .map_err(debug)?
                );
                assert_eq!(lock, before);
                clock.advance(1_000);
            }
            Ok(())
        })
    }

    #[test]
    fn readmission_current_session_lease_and_scope_refusals_preserve_lock() -> TestResult {
        run("pvp_readmit_fence_refusals", async |h| {
            stored(h, Some(60_000), Some(60_000), Some(60_000)).await?;
            let clock = VirtualOwnerClock::new(OwnerTime::from_micros(1_000));
            let mut positive = CombatLock::default();
            assert!(
                call(h, &mut positive, &clock, fence(1)?)
                    .await?
                    .map_err(debug)?
            );
            let mut variants = Vec::new();
            let mut changed = fence(1)?;
            changed.character_id = crate::domain::CharacterId::from_bytes(id(99)).map_err(debug)?;
            variants.push(changed);
            let mut changed = fence(1)?;
            changed.character_lease_generation = 0;
            variants.push(changed);
            let mut changed = fence(1)?;
            changed.game_session_id = GameSessionId::decode(&id(99)).map_err(debug)?;
            variants.push(changed);
            let mut changed = fence(1)?;
            changed.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
            variants.push(changed);
            let mut changed = fence(1)?;
            changed.character_lease_generation = 2;
            variants.push(changed);
            let mut changed = fence(1)?;
            changed.scope_ownership_generation = ScopeOwnershipGeneration::new(2).map_err(debug)?;
            variants.push(changed);
            let mut changed = fence(1)?;
            changed.runtime_scope = RuntimeScopeRefV1::channel(
                WorldId::decode(&id(99)).map_err(debug)?,
                ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
            );
            variants.push(changed);
            let mut changed = fence(1)?;
            changed.runtime_scope = RuntimeScopeRefV1::channel(
                WorldId::decode(&id(WORLD)).map_err(debug)?,
                ChannelId::decode(&id(99)).map_err(debug)?,
            );
            variants.push(changed);
            for changed in variants {
                let mut lock = prior_lock();
                let before = lock;
                assert!(matches!(
                    call(h, &mut lock, &clock, changed).await?,
                    Err(PvpReadmissionError::Read(_))
                ));
                assert_eq!(lock, before);
            }
            // The actual read uses item_fence(), so CharacterRevision is not a read invariant.
            assert!(
                call(h, &mut positive, &clock, fence(999)?)
                    .await?
                    .map_err(debug)?
            );
            Ok(())
        })
    }

    #[test]
    fn readmission_revoked_actual_node_refuses_with_same_expected_binding() -> TestResult {
        run("pvp_readmit_node_revoked", async |h| {
            stored(h, Some(60_000), Some(60_000), Some(60_000)).await?;
            let clock = VirtualOwnerClock::new(OwnerTime::from_micros(1_000));
            let mut positive = CombatLock::default();
            assert!(
                call(h, &mut positive, &clock, fence(1)?)
                    .await?
                    .map_err(debug)?
            );
            h.root
                .revoke_node_registration(h.node.fact())
                .await
                .map_err(debug)?;
            let mut lock = prior_lock();
            let before = lock;
            assert!(matches!(
                call(h, &mut lock, &clock, fence(1)?).await?,
                Err(PvpReadmissionError::Read(_))
            ));
            assert_eq!(lock, before);
            Ok(())
        })
    }

    #[test]
    fn readmission_authority_from_another_actual_root_cannot_mutate_lock() -> TestResult {
        run("pvp_readmit_root_binding", async |h| {
            stored(h, Some(60_000), Some(60_000), Some(60_000)).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let other_root = DurabilityRoot::connect_test_runtime(&h.database.url)?;
            assert!(other_root.maintain_ready_once().await?);
            let clock = VirtualOwnerClock::new(OwnerTime::from_micros(1_000));
            let mut positive = CombatLock::default();
            assert!(
                positive
                    .restore_pvp_readmission(&h.root, &authority, &h.node, fence(1)?, &clock)
                    .await
                    .map_err(debug)?
            );
            let mut lock = prior_lock();
            let before = lock;
            assert!(matches!(
                lock.restore_pvp_readmission(&other_root, &authority, &h.node, fence(1)?, &clock)
                    .await,
                Err(PvpReadmissionError::Read(_))
            ));
            assert_eq!(lock, before);
            Ok(())
        })
    }

    #[test]
    fn readmission_actual_database_read_error_preserves_all_prior_slots() -> TestResult {
        run("pvp_readmit_db_error", async |h| {
            stored(h, Some(60_000), Some(60_000), Some(60_000)).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let clock = VirtualOwnerClock::new(OwnerTime::from_micros(1_000));
            let mut positive = CombatLock::default();
            assert!(
                positive
                    .restore_pvp_readmission(&h.root, &authority, &h.node, fence(1)?, &clock)
                    .await
                    .map_err(debug)?
            );
            // Disposable fixture DB only: cause a genuine owner's query failure after the
            // independently successful control. No production schema or shared source changes.
            sqlx::query(
                "ALTER TABLE game_character_pvp_state RENAME TO fixture_unavailable_pvp_state",
            )
            .execute(&h.pool)
            .await?;
            let mut lock = prior_lock();
            let before = lock;
            assert!(matches!(
                lock.restore_pvp_readmission(&h.root, &authority, &h.node, fence(1)?, &clock)
                    .await,
                Err(PvpReadmissionError::Read(_))
            ));
            assert_eq!(lock, before);
            Ok(())
        })
    }

    #[test]
    fn readmission_later_deadline_overflow_is_atomic_for_all_three_slots() -> TestResult {
        run("pvp_readmit_checked_overflow", async |h| {
            stored(h, Some(10_000), Some(20_000), Some(60_000)).await?;
            let clock = VirtualOwnerClock::new(OwnerTime::from_micros(u64::MAX - 40_000_000));
            let mut lock = prior_lock();
            let before = lock;
            assert!(matches!(
                call(h, &mut lock, &clock, fence(1)?).await?,
                Err(PvpReadmissionError::DeadlineOverflow)
            ));
            assert_eq!(lock, before);
            Ok(())
        })
    }

    #[test]
    fn readmission_samples_existing_owner_clock_after_the_blocked_native_read() -> TestResult {
        run("pvp_readmit_clock_order", async |h| {
            stored(h, Some(60_000), None, None).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let clock = VirtualOwnerClock::new(OwnerTime::from_micros(1_000));
            let mut lock = CombatLock::default();
            let mut blocker = h.pool.begin().await?;
            sqlx::query("LOCK TABLE game_character_pvp_state IN ACCESS EXCLUSIVE MODE")
                .execute(&mut *blocker)
                .await?;
            {
                let pending =
                    lock.restore_pvp_readmission(&h.root, &authority, &h.node, fence(1)?, &clock);
                tokio::pin!(pending);
                if let Ok(result) =
                    tokio::time::timeout(std::time::Duration::from_millis(20), pending.as_mut())
                        .await
                {
                    return Err(
                        format!("native read bypassed actual relation lock: {result:?}").into(),
                    );
                }
                clock.advance(100_000_000); // Existing OwnerClock DATA, no fake provider.
                blocker.commit().await?;
                assert!(pending.await.map_err(debug)?);
            }
            assert!(lock.blocks_logout(SimTime::from_micros(101_000_000), None));
            assert!(!lock.blocks_logout(SimTime::from_micros(161_000_000), None));
            Ok(())
        })
    }
}
