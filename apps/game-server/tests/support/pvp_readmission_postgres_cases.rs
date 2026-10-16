//! PvP readmission cases routed through the existing canonical authority PostgreSQL root.
#![allow(clippy::expect_used)]
#[allow(dead_code)]
#[path = "../../src/combat/pvp/lock.rs"]
mod pvp_readmission_lock;

mod readmission_cases {
    use super::pvp_readmission_lock::{CombatLock, PvpReadmissionError};
    use crate::bestiary_postgres_harness::{
        CHANNEL, CHARACTER, Harness, TestResult, WORLD, configured_admin, debug, fence, id, runtime,
    };
    use crate::durability::DurabilityRoot;
    use crate::durability::character_progression::CurrentCharacterGameplayFence;
    use crate::foundation::{
        ChannelId, ConnectionGeneration, GameSessionId, RuntimeScopeRefV1,
        ScopeOwnershipGeneration, WorldId,
    };
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

    fn apply_completed_read(
        lock: &mut CombatLock,
        read: Result<
            Option<crate::durability::pvp_state::StoredPvpState>,
            crate::durability::character_progression::CharacterProgressionError,
        >,
    ) -> Result<bool, PvpReadmissionError> {
        // No clock is sampled across the I/O boundary; the synchronous owner apply owns it.
        let clock = crate::foundation::owner_timer::VirtualOwnerClock::new(
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(1_000),
        );
        lock.restore_pvp_readmission(
            read.map(|state| state.map(|row| row.readmission_remaining)),
            &clock,
        )
    }

    async fn call(
        h: &Harness,
        lock: &mut CombatLock,
        expected: CurrentCharacterGameplayFence,
    ) -> TestResult<Result<bool, PvpReadmissionError>> {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let read = h.root.read_pvp_state(&authority, &h.node, expected).await;
        Ok(apply_completed_read(lock, read))
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
    fn readmission_keeps_each_absolute_deadline_bound_to_its_database_sample() -> TestResult {
        run("pvp_readmit_database_clock", async |h| {
            stored(h, Some(60_000), Some(120_000), Some(180_000)).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let state = h
                .root
                .read_pvp_state(&authority, &h.node, fence(1)?)
                .await
                .map_err(debug)?
                .ok_or("stored row missing")?;
            let sample = state.readmission_clock.sampled_at.get();
            for (deadline, remaining, micros) in [
                (
                    state.readmission_clock.logout_until,
                    state.logout_block_remaining_ms,
                    state.readmission_remaining.logout,
                ),
                (
                    state.readmission_clock.pz_until,
                    state.pz_block_remaining_ms,
                    state.readmission_remaining.pz,
                ),
                (
                    state.readmission_clock.kill_until,
                    state.kill_block_remaining_ms,
                    state.readmission_remaining.kill,
                ),
            ] {
                let until = deadline.ok_or("active database deadline missing")?.get();
                assert!(until > sample);
                // Both values must describe the very same database sample, including rounding.
                assert_eq!(remaining, Some((until - sample).div_ceil(1_000)));
                assert_eq!(micros.get(), until - sample);
            }
            let mut lock = prior_lock();
            let before = lock;
            assert!(matches!(call(h, &mut lock, fence(1)?).await?, Ok(true)));
            assert_eq!(lock, before);
            assert_eq!(h.root_revision().await?, "1");
            Ok(())
        })
    }

    #[test]
    fn readmission_no_row_and_expired_row_never_clear_prior_locks() -> TestResult {
        run("pvp_readmit_absent_expired", async |h| {
            let mut lock = prior_lock();
            let before = lock;
            assert!(!call(h, &mut lock, fence(1)?).await?.map_err(debug)?);
            assert_eq!(lock, before);
            stored(h, Some(0), Some(-1), None).await?;
            assert!(call(h, &mut lock, fence(1)?).await?.map_err(debug)?);
            assert_eq!(lock, before);
            Ok(())
        })
    }

    #[test]
    fn readmission_database_sample_is_taken_after_actual_relation_lock_wait() -> TestResult {
        run("pvp_readmit_tx_clock_wait", async |h| {
            stored(h, Some(60_000), None, None).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut blocker = h.pool.begin().await?;
            sqlx::query("LOCK TABLE game_character_pvp_state IN ACCESS EXCLUSIVE MODE")
                .execute(&mut *blocker)
                .await?;
            let pending = h.root.read_pvp_state(&authority, &h.node, fence(1)?);
            tokio::pin!(pending);
            if let Ok(result) =
                tokio::time::timeout(std::time::Duration::from_millis(20), pending.as_mut()).await
            {
                return Err(format!("native read bypassed actual lock: {result:?}").into());
            }
            let releasing_at: i64 = sqlx::query_scalar(
                "SELECT (extract(epoch FROM clock_timestamp()) * 1000000)::bigint",
            )
            .fetch_one(&mut *blocker)
            .await?;
            blocker.commit().await?;
            let state = pending.await.map_err(debug)?.ok_or("stored row missing")?;
            assert!(state.readmission_clock.sampled_at.get() >= u64::try_from(releasing_at)?);
            Ok(())
        })
    }

    #[test]
    fn readmission_current_session_lease_and_scope_refusals_preserve_lock() -> TestResult {
        run("pvp_readmit_fence_refusals", async |h| {
            stored(h, Some(60_000), Some(60_000), Some(60_000)).await?;
            let mut positive = CombatLock::default();
            assert!(matches!(call(h, &mut positive, fence(1)?).await?, Ok(true)));
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
                    call(h, &mut lock, changed).await?,
                    Err(PvpReadmissionError::Read(_))
                ));
                assert_eq!(lock, before);
            }
            // The actual read uses item_fence(), so CharacterRevision is not a read invariant.
            assert!(matches!(
                call(h, &mut positive, fence(999)?).await?,
                Ok(true)
            ));
            Ok(())
        })
    }

    #[test]
    fn readmission_revoked_actual_node_refuses_with_same_expected_binding() -> TestResult {
        run("pvp_readmit_node_revoked", async |h| {
            stored(h, Some(60_000), Some(60_000), Some(60_000)).await?;
            let mut positive = CombatLock::default();
            assert!(matches!(call(h, &mut positive, fence(1)?).await?, Ok(true)));
            h.root
                .revoke_node_registration(h.node.fact())
                .await
                .map_err(debug)?;
            let mut lock = prior_lock();
            let before = lock;
            assert!(matches!(
                call(h, &mut lock, fence(1)?).await?,
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
            let mut positive = CombatLock::default();
            assert!(matches!(
                apply_completed_read(
                    &mut positive,
                    h.root.read_pvp_state(&authority, &h.node, fence(1)?).await
                ),
                Ok(true)
            ));
            let mut lock = prior_lock();
            let before = lock;
            assert!(matches!(
                apply_completed_read(
                    &mut lock,
                    other_root
                        .read_pvp_state(&authority, &h.node, fence(1)?)
                        .await
                ),
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
            let mut positive = CombatLock::default();
            assert!(matches!(
                apply_completed_read(
                    &mut positive,
                    h.root.read_pvp_state(&authority, &h.node, fence(1)?).await
                ),
                Ok(true)
            ));
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
                apply_completed_read(
                    &mut lock,
                    h.root.read_pvp_state(&authority, &h.node, fence(1)?).await
                ),
                Err(PvpReadmissionError::Read(_))
            ));
            assert_eq!(lock, before);
            Ok(())
        })
    }
}
