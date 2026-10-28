use super::*;

fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}
fn key() -> BossClockKey {
    BossClockKey {
        world: WorldId::decode(&id(42)).expect("world"),
        channel: ChannelId::decode(&id(43)).expect("channel"),
        spawn: "boss.spawn.fixture".into(),
    }
}
fn generation(value: u64) -> ScopeOwnershipGeneration {
    ScopeOwnershipGeneration::new(value).expect("generation")
}
fn due() -> BossClock {
    BossClock {
        cycle: 7,
        state: BossClockState::Due,
        reset_epoch: 10,
        due_at_unix_ms: Some(1000),
        owner_generation: None,
    }
}
fn alive() -> BossClock {
    plan_claim(&due(), generation(1), 1000)
        .expect("valid")
        .expect("due")
}

#[test]
fn due_boundary_and_ended_owner_recovery_never_advance_cycle() {
    let clock = due();
    assert!(
        plan_claim(&clock, generation(1), 999)
            .expect("future")
            .is_none()
    );
    let claimed = plan_claim(&clock, generation(1), 1000)
        .expect("boundary")
        .expect("due");
    assert_eq!(claimed.cycle, 7);
    assert!(
        plan_claim(&claimed, generation(1), 2000)
            .expect("already own")
            .is_none()
    );
    let recovered = plan_claim(&claimed, generation(2), 2000)
        .expect("new owner")
        .expect("recover");
    assert_eq!(recovered.owner_generation, Some(generation(2)));
    assert_eq!(recovered.cycle, 7);
    assert!(matches!(
        plan_claim(&recovered, generation(1), 2000),
        Err(BossClockError::AuthorityRejected)
    ));
}

#[test]
fn reset_overwrites_every_previous_state_before_any_recovery_once_per_epoch() {
    let inactive = BossClock {
        state: BossClockState::Inactive,
        due_at_unix_ms: None,
        ..due()
    };
    for old in [due(), alive(), inactive] {
        let no = plan_reset(&key(), &old, 11, 9000, 0, 1)
            .expect("reset")
            .expect("new epoch");
        assert_eq!(no.state, BossClockState::Inactive);
        assert_eq!(no.owner_generation, None);
        assert_eq!(no.reset_epoch, 11);
        assert!(
            plan_claim(&no, generation(2), 10000)
                .expect("inactive")
                .is_none()
        );
        assert!(
            plan_reset(&key(), &no, 11, 9999, 1, 1)
                .expect("same epoch")
                .is_none()
        );
        let yes = plan_reset(&key(), &old, 11, 9000, 1, 1)
            .expect("reset")
            .expect("new epoch");
        assert_eq!(yes.state, BossClockState::Due);
        assert_eq!(yes.due_at_unix_ms, Some(9000));
        assert_eq!(yes.cycle, old.cycle);
        assert!(
            plan_claim(&yes, generation(2), 8999)
                .expect("future")
                .is_none()
        );
        assert!(
            plan_claim(&yes, generation(2), 9000)
                .expect("now")
                .is_some()
        );
    }
}

#[test]
fn reset_death_stays_inactive_until_the_next_epoch() {
    let dead = plan_reset_death(&alive()).expect("death proposal");
    assert_eq!(dead.cycle, 8);
    assert_eq!(dead.reset_epoch, 10);
    assert!(
        plan_reset(&key(), &dead, 10, 1000, 1, 1)
            .expect("same epoch")
            .is_none()
    );
    assert!(
        plan_claim(&dead, generation(2), 999999)
            .expect("inactive")
            .is_none()
    );
    assert_eq!(
        plan_reset(&key(), &dead, 11, 2000, 1, 1)
            .expect("new epoch")
            .expect("draw")
            .state,
        BossClockState::Due
    );
}

#[test]
fn native_rng_is_bound_to_clock_cycle_and_purpose_with_checked_death_time() {
    let old = alive();
    let next = plan_respawn_death(&key(), &old, 2000, 5, 5).expect("fixed interval");
    assert_eq!(next.due_at_unix_ms, Some(7000));
    assert_eq!(next.cycle, 8);
    assert_eq!(
        next,
        plan_respawn_death(&key(), &old, 2000, 5, 5).expect("same proposal")
    );
    for cycle in 0..128 {
        let clock = BossClock {
            cycle,
            ..old.clone()
        };
        let next = plan_respawn_death(&key(), &clock, 2000, 2, 5).expect("range");
        assert!((4000..=7000).contains(&next.due_at_unix_ms.expect("due")));
    }
    let draw0 = draw(&key(), 8, "boss_respawn").expect("native decision");
    assert_ne!(draw0, draw(&key(), 9, "boss_respawn").expect("cycle"));
    assert_ne!(draw0, draw(&key(), 8, "boss_at_reset").expect("purpose"));
    let mut other = key();
    other.channel = ChannelId::decode(&id(44)).expect("other channel");
    assert_ne!(draw0, draw(&other, 8, "boss_respawn").expect("channel"));
    other = key();
    other.world = WorldId::decode(&id(44)).expect("other world");
    assert_ne!(draw0, draw(&other, 8, "boss_respawn").expect("world"));
    other = key();
    other.spawn.push('x');
    assert_ne!(draw0, draw(&other, 8, "boss_respawn").expect("spawn"));
    for (death, min, max) in [
        (i64::MAX, 1, 1),
        (0, 5, 4),
        (0, i64::MAX as u64, i64::MAX as u64),
        (-1, 0, 0),
    ] {
        assert!(plan_respawn_death(&key(), &old, death, min, max).is_err());
    }
    assert!(
        plan_respawn_death(
            &key(),
            &BossClock {
                cycle: u64::MAX,
                ..old
            },
            0,
            0,
            0
        )
        .is_err()
    );
}

#[test]
fn malformed_shapes_nonmonotonic_epoch_and_chance_fail_closed() {
    for bad in [
        BossClock {
            owner_generation: Some(generation(1)),
            ..due()
        },
        BossClock {
            due_at_unix_ms: None,
            ..due()
        },
        BossClock {
            due_at_unix_ms: Some(-1),
            ..due()
        },
        BossClock {
            state: BossClockState::Alive,
            ..due()
        },
    ] {
        assert!(bad.validate().is_err());
    }
    for (epoch, time, numerator, denominator) in [
        (9, 1000, 1, 1),
        (11, -1, 1, 1),
        (11, 1000, 2, 1),
        (11, 1000, 1, 0),
    ] {
        assert!(plan_reset(&key(), &due(), epoch, time, numerator, denominator).is_err());
    }
}

// Private schema qualification. Native assignment/recovery APIs provide the independent owner
// facts; these fixture clock INSERTs never create authority and do not install an actor.

use crate::bestiary_postgres_harness as native_fixture;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

async fn qualify_clock_storage(fixture: &native_fixture::Harness) -> TestResult {
    use crate::durability::runtime_scope_assignment as native;
    use crate::durability::runtime_scope_assignment::{BootstrapSecret, NodeRegistrationFact};
    use sqlx::Connection;
    let root = fixture.root.clone();
    let writer = &fixture.writer;
    let pool = fixture.pool.clone();
    let url = fixture.database.url.clone();
    let clock_key = key();
    let scope = native_fixture::scope()?;
    let seal = fixture
        .recovery
        .seal_current()
        .map_err(|e| format!("{e:?}"))?;
    let recovery = seal.record().clone();
    let native_proof = fixture.node.clone();
    let proof = native_proof.clone();
    sqlx::query("INSERT INTO game_boss_spawn_clocks VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,7,1,10,0,NULL)").bind(clock_key.world.as_bytes().as_slice()).bind(clock_key.channel.as_bytes().as_slice()).bind(&clock_key.spawn).execute(&pool).await?;
    let expected = BossClock {
        due_at_unix_ms: Some(0),
        ..due()
    };
    let mut missing_key = clock_key.clone();
    missing_key.spawn.push_str(".missing");
    let mut tx = pool.begin().await?;
    assert!(matches!(
        claim_respawn_clock_in_transaction(
            &mut tx,
            &recovery,
            &proof,
            &missing_key,
            generation(1),
            &expected
        )
        .await,
        Err(BossClockError::MissingClock)
    ));
    tx.rollback().await?;
    // Other clock rows are valid too: only their independent owner binding is absent.
    for (world, channel) in [
        (
            clock_key.world,
            ChannelId::decode(&id(44)).map_err(|e| format!("{e:?}"))?,
        ),
        (
            WorldId::decode(&id(44)).map_err(|e| format!("{e:?}"))?,
            clock_key.channel,
        ),
    ] {
        sqlx::query("INSERT INTO game_boss_spawn_clocks VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,7,1,10,0,NULL)").bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(&clock_key.spawn).execute(&pool).await?;
    }
    // Every negative keeps the valid clock and independent node/session-free owner
    // fixture. Exactly one authority binding is changed; all transactions roll back.
    for bad in 0..5 {
        let mut changed_key = clock_key.clone();
        let mut changed_recovery = recovery.clone();
        let mut changed_proof = proof.clone();
        let mut changed_generation = generation(1);
        match bad {
            0 => changed_generation = generation(2),
            1 => changed_key.channel = ChannelId::decode(&id(44)).map_err(|e| format!("{e:?}"))?,
            2 => {
                changed_proof = NodeIncarnationProof::new(
                    NodeRegistrationFact::new(
                        native_proof.fact().node_id(),
                        native_proof.fact().registration_revision(),
                    ),
                    BootstrapSecret::from_bytes([2; 32]),
                )
            }
            3 => changed_recovery.authority_scope_id = "another-authority".into(),
            _ => changed_key.world = WorldId::decode(&id(44)).map_err(|e| format!("{e:?}"))?,
        }
        let mut tx = pool.begin().await?;
        let rejected = claim_respawn_clock_in_transaction(
            &mut tx,
            &changed_recovery,
            &changed_proof,
            &changed_key,
            changed_generation,
            &expected,
        )
        .await;
        if bad == 3 {
            assert!(matches!(rejected, Err(BossClockError::Unavailable(_))));
        } else {
            assert!(matches!(rejected, Err(BossClockError::AuthorityRejected)));
        }
        tx.rollback().await?;
        let mut tx = pool.begin().await?;
        assert_eq!(
            read_clock(&mut tx, &clock_key).await?,
            Some(expected.clone())
        );
        tx.rollback().await?;
    }
    // A current materialized assignment is insufficient when its retained native source
    // high-water/history disagrees. Only this provenance invariant changes; roll it back.
    let mut tx = pool.begin().await?;
    lock_admission_relations(&mut tx).await?;
    sqlx::query("UPDATE game_runtime_scope_assignment_writer SET source_revision_high_water=source_revision_high_water+1 WHERE writer_id=1")
        .execute(&mut *tx).await?;
    let drift = claim_respawn_clock_in_transaction(
        &mut tx,
        &recovery,
        &proof,
        &clock_key,
        generation(1),
        &expected,
    )
    .await;
    tx.rollback().await?;
    if !matches!(drift, Err(BossClockError::InvalidStoredState)) {
        return Err("boss claim accepted native assignment history/high-water drift".into());
    }
    // A pending claim rolled back is not a durable ALIVE or an installation receipt.
    let mut tx = pool.begin().await?;
    assert!(matches!(
        claim_respawn_clock_in_transaction(
            &mut tx,
            &recovery,
            &proof,
            &clock_key,
            generation(1),
            &expected
        )
        .await?,
        BossClockClaim::Claimed(_)
    ));
    tx.rollback().await?;
    let mut tx = pool.begin().await?;
    assert_eq!(
        read_clock(&mut tx, &clock_key).await?,
        Some(expected.clone())
    );
    tx.rollback().await?;
    let mut handles = Vec::new();
    for _ in 0..2 {
        let (pool, proof, key, recovery, expected) = (
            pool.clone(),
            proof.clone(),
            clock_key.clone(),
            recovery.clone(),
            expected.clone(),
        );
        handles.push(tokio::spawn(async move {
            let mut tx = pool.begin().await?;
            let out = claim_respawn_clock_in_transaction(
                &mut tx,
                &recovery,
                &proof,
                &key,
                generation(1),
                &expected,
            )
            .await?;
            tx.commit().await?;
            Ok::<_, BossClockError>(out)
        }));
    }
    let first = handles.remove(0).await??;
    let second = handles.remove(0).await??;
    assert_eq!(
        [&first, &second]
            .iter()
            .filter(|out| matches!(out, BossClockClaim::Claimed(_)))
            .count(),
        1
    );
    assert_eq!(
        [&first, &second]
            .iter()
            .filter(|out| matches!(out, BossClockClaim::CompareFailed))
            .count(),
        1
    );
    let mut tx = pool.begin().await?;
    let historical = read_clock(&mut tx, &clock_key)
        .await?
        .ok_or("missing clock")?;
    assert_eq!(
        claim_respawn_clock_in_transaction(
            &mut tx,
            &recovery,
            &proof,
            &clock_key,
            generation(1),
            &historical
        )
        .await?,
        BossClockClaim::Unchanged
    );
    assert_eq!(
        claim_respawn_clock_in_transaction(
            &mut tx,
            &recovery,
            &proof,
            &clock_key,
            generation(1),
            &expected
        )
        .await?,
        BossClockClaim::CompareFailed
    );
    for stale in [
        BossClock {
            cycle: 8,
            ..historical.clone()
        },
        BossClock {
            reset_epoch: 11,
            ..historical.clone()
        },
        BossClock {
            owner_generation: Some(generation(2)),
            ..historical.clone()
        },
    ] {
        assert!(!compare_and_set_clock(&mut tx, &clock_key, &stale, &historical).await?);
    }
    tx.commit().await?;
    // Losing/uncertain caller can read history, but ALIVE never proves actor installation.
    let predecessor = root
        .read_runtime_scope_predecessor(scope)
        .await
        .map_err(|e| format!("{e:?}"))?
        .ok_or("missing predecessor")?;
    let replaced = writer
        .submit(&native::AssignmentRequest {
            operation_key: native::OperationKey::from_bytes([8; 32]),
            actor: native::ControlActor::new("oteryn_test_admin").map_err(|e| format!("{e:?}"))?,
            command: native::AssignmentCommand::Replace {
                scope,
                predecessor,
                target: native_proof.fact(),
            },
        })
        .await
        .map_err(|e| format!("{e:?}"))?;
    let native::AssignmentOutcome::Committed(replaced) = replaced else {
        return Err("replace rejected".into());
    };
    assert_eq!(replaced.assignment.ownership_generation, 2);
    let mut tx = pool.begin().await?;
    assert!(matches!(
        claim_respawn_clock_in_transaction(
            &mut tx,
            &recovery,
            &proof,
            &clock_key,
            generation(1),
            &historical
        )
        .await,
        Err(BossClockError::AuthorityRejected)
    ));
    tx.rollback().await?;
    let mut tx = pool.begin().await?;
    let recovered = claim_respawn_clock_in_transaction(
        &mut tx,
        &recovery,
        &proof,
        &clock_key,
        generation(2),
        &historical,
    )
    .await?;
    let BossClockClaim::Claimed(recovered) = recovered else {
        return Err("new owner failed to recover".into());
    };
    assert_eq!(recovered.cycle, 7);
    assert_eq!(recovered.owner_generation, Some(generation(2)));
    tx.commit().await?;
    root.revoke_node_registration(native_proof.fact())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let mut tx = pool.begin().await?;
    assert!(matches!(
        claim_respawn_clock_in_transaction(
            &mut tx,
            &recovery,
            &proof,
            &clock_key,
            generation(2),
            &recovered
        )
        .await,
        Err(BossClockError::AuthorityRejected)
    ));
    tx.rollback().await?;
    let mut reloaded = sqlx::PgConnection::connect(&url).await?;
    let mut tx = reloaded.begin().await?;
    assert_eq!(read_clock(&mut tx, &clock_key).await?, Some(recovered));
    tx.rollback().await?;
    reloaded.close().await?;
    Ok(())
}

#[test]
fn boss_clock_native_owner_cas_and_restart_storage_postgres() -> TestResult {
    let Some(admin) = native_fixture::configured_admin() else {
        return Ok(());
    };
    native_fixture::runtime()?.block_on(async move {
        let fixture = native_fixture::Harness::create(admin, "boss_clocks", false).await?;
        eprintln!(
            "Canonical PostgreSQL 18.6; genuine native Root/node/assignment/recovery fixture"
        );
        let result = qualify_clock_storage(&fixture).await;
        fixture.cleanup().await?;
        result
    })
}

// Migration 0246 supplies the actual shared schema. Fixture rows are not proof
// of a current reset, boosted boss, admission or committed boss death.

fn reward_key(boss: &str) -> BossRewardKey {
    BossRewardKey::new(
        CharacterId::from_bytes(id(native_fixture::CHARACTER)).expect("character"),
        boss,
    )
    .expect("qualified boss key")
}

#[test]
fn boss_reward_storage_rejects_unqualified_keys_and_time_overflow() {
    for name in [
        "",
        "boss_without_namespace",
        "oteryn:boss with spaces",
        "oteryn:test-only",
    ] {
        assert!(BossRewardKey::new(reward_key("oteryn:boss.alpha").character, name).is_err());
    }
    assert_eq!(reward_deadline(1000, 0).expect("zero interval"), 1000);
    assert_eq!(
        reward_deadline(1000, 30 * 24 * 60 * 60).expect("30 days"),
        2_592_001_000
    );
    for (now, seconds) in [(-1, 0), (i64::MAX, 1), (0, u64::MAX)] {
        assert!(matches!(
            reward_deadline(now, seconds),
            Err(BossRewardStateError::InvalidInput)
        ));
    }
}

async fn qualify_reward_claims(fixture: &native_fixture::Harness) -> TestResult {
    let seal = fixture
        .recovery
        .seal_current()
        .map_err(native_fixture::debug)?;
    let recovery = seal.record().clone();
    let fence = native_fixture::item_fence()?;
    let boss = "oteryn:boss.reward_claim";
    let key = reward_key(boss);

    // A failure later in the SAME transaction must release both kinds of claim.
    let mut tx = fixture.pool.begin().await?;
    assert_eq!(
        claim_respawn_eligibility_in_transaction(
            &mut tx,
            &recovery,
            &fixture.node,
            &fence,
            boss,
            3600
        )
        .await?,
        BossEligibilityClaim::Claimed
    );
    assert_eq!(
        claim_reference_eligibility_data(&mut tx, &key, id(90)).await?,
        BossEligibilityClaim::Claimed
    );
    assert!(sqlx::query("SELECT 1/0").execute(&mut *tx).await.is_err());
    tx.rollback().await?;
    assert_eq!(
        fixture
            .count("game_character_boss_open_world_eligibility")
            .await?,
        0
    );
    assert_eq!(fixture.count("game_character_boss_eligibility").await?, 0);

    // Genuine independent native fences: exactly one changed fact per negative.
    for bad in 0..5 {
        let mut changed = fence;
        match bad {
            0 => {
                changed.connection_generation = crate::foundation::ConnectionGeneration::new(2)
                    .map_err(native_fixture::debug)?
            }
            1 => changed.character_lease_generation = 2,
            2 => changed.scope_ownership_generation = generation(2),
            3 => {
                changed.runtime_scope = crate::foundation::RuntimeScopeRefV1::channel(
                    WorldId::decode(&id(46)).map_err(native_fixture::debug)?,
                    ChannelId::decode(&id(native_fixture::CHANNEL))
                        .map_err(native_fixture::debug)?,
                )
            }
            _ => {
                changed.runtime_scope = crate::foundation::RuntimeScopeRefV1::instance(
                    WorldId::decode(&id(native_fixture::WORLD)).map_err(native_fixture::debug)?,
                    id(46),
                )
                .map_err(native_fixture::debug)?
            }
        }
        let mut tx = fixture.pool.begin().await?;
        assert!(matches!(
            claim_respawn_eligibility_in_transaction(
                &mut tx,
                &recovery,
                &fixture.node,
                &changed,
                boss,
                3600
            )
            .await,
            Err(BossRewardStateError::AuthorityRejected)
        ));
        tx.rollback().await?;
        assert_eq!(
            fixture
                .count("game_character_boss_open_world_eligibility")
                .await?,
            0
        );
    }
    let mut wrong_recovery = recovery.clone();
    wrong_recovery.authority_scope_id.push_str("-stale");
    let mut tx = fixture.pool.begin().await?;
    assert!(matches!(
        claim_respawn_eligibility_in_transaction(
            &mut tx,
            &wrong_recovery,
            &fixture.node,
            &fence,
            boss,
            3600
        )
        .await,
        Err(BossRewardStateError::Unavailable(_))
    ));
    tx.rollback().await?;
    let mut tx = fixture.pool.begin().await?;
    sqlx::query("UPDATE game_runtime_scope_assignment_writer SET source_revision_high_water=source_revision_high_water+1 WHERE writer_id=1")
        .execute(&mut *tx).await?;
    assert!(matches!(
        claim_respawn_eligibility_in_transaction(
            &mut tx,
            &recovery,
            &fixture.node,
            &fence,
            boss,
            3600
        )
        .await,
        Err(BossRewardStateError::InvalidStoredState)
    ));
    tx.rollback().await?;

    // The native admitted-session fence is the accepted boundary: terminal rows,
    // withdrawn admission and a wrong node secret each reject independently.
    for mutation in [
        "UPDATE game_durability_reconnect_sessions SET session_state=3",
        "UPDATE game_durability_admission_character_guards SET eligible=false",
    ] {
        let mut tx = fixture.pool.begin().await?;
        sqlx::query(sqlx::AssertSqlSafe(mutation.to_owned()))
            .execute(&mut *tx)
            .await?;
        assert!(matches!(
            claim_respawn_eligibility_in_transaction(
                &mut tx,
                &recovery,
                &fixture.node,
                &fence,
                boss,
                3600
            )
            .await,
            Err(BossRewardStateError::AuthorityRejected)
        ));
        tx.rollback().await?;
    }
    let fact = fixture.node.fact();
    let wrong_secret = NodeIncarnationProof::new(
        crate::durability::runtime_scope_assignment::NodeRegistrationFact::new(
            fact.node_id(),
            fact.registration_revision(),
        ),
        crate::durability::runtime_scope_assignment::BootstrapSecret::from_bytes([2; 32]),
    );
    let mut tx = fixture.pool.begin().await?;
    assert!(matches!(
        claim_respawn_eligibility_in_transaction(
            &mut tx,
            &recovery,
            &wrong_secret,
            &fence,
            boss,
            3600
        )
        .await,
        Err(BossRewardStateError::AuthorityRejected)
    ));
    tx.rollback().await?;
    assert_eq!(
        fixture
            .count("game_character_boss_open_world_eligibility")
            .await?,
        0
    );

    // Singleton concurrency using the same native Channel fence twice: one pending
    // claim wins, the other observes next_eligible_at. This is NOT qualification of
    // an actual cross-channel death/transfer caller. The key intentionally omits Channel.
    let mut tasks = Vec::new();
    for _ in 0..2 {
        let (pool, recovery, node) = (fixture.pool.clone(), recovery.clone(), fixture.node.clone());
        tasks.push(tokio::spawn(async move {
            let mut tx = pool.begin().await?;
            let outcome = claim_respawn_eligibility_in_transaction(
                &mut tx, &recovery, &node, &fence, boss, 3600,
            )
            .await?;
            tx.commit().await?;
            Ok::<_, BossRewardStateError>(outcome)
        }));
    }
    let a = tasks.remove(0).await??;
    let b = tasks.remove(0).await??;
    assert_eq!(
        [a, b]
            .iter()
            .filter(|&&v| v == BossEligibilityClaim::Claimed)
            .count(),
        1
    );
    assert_eq!(
        [a, b]
            .iter()
            .filter(|&&v| v == BossEligibilityClaim::Ineligible)
            .count(),
        1
    );
    assert_eq!(
        fixture
            .count("game_character_boss_open_world_eligibility")
            .await?,
        1
    );
    assert_eq!(fixture.root_revision().await?, "1");

    // Drop the connection, reopen from the durable store, and verify that a new
    // transaction cannot bypass the window or repeat a firing's reference credit.
    let pool = sqlx::PgPool::connect(&fixture.database.url).await?;
    let mut tx = pool.begin().await?;
    assert_eq!(
        claim_respawn_eligibility_in_transaction(
            &mut tx,
            &recovery,
            &fixture.node,
            &fence,
            boss,
            3600
        )
        .await?,
        BossEligibilityClaim::Ineligible
    );
    assert_eq!(
        claim_reference_eligibility_data(&mut tx, &key, id(90)).await?,
        BossEligibilityClaim::Claimed
    );
    tx.commit().await?;
    let mut tx = pool.begin().await?;
    assert_eq!(
        claim_reference_eligibility_data(&mut tx, &key, id(90)).await?,
        BossEligibilityClaim::Ineligible
    );
    assert_eq!(
        claim_reference_eligibility_data(&mut tx, &key, id(91)).await?,
        BossEligibilityClaim::Claimed
    );
    tx.commit().await?;
    pool.close().await;
    Ok(())
}

async fn qualify_reward_boundaries(fixture: &native_fixture::Harness) -> TestResult {
    let key = reward_key("oteryn:boss.respawn_boundary");
    let mut tx = fixture.pool.begin().await?;
    assert_eq!(
        claim_open_world_eligibility_data(
            &mut tx,
            &key,
            1000,
            OpenWorldEligibilityData::Respawn { min_s: 5 }
        )
        .await?,
        BossEligibilityClaim::Claimed
    );
    tx.commit().await?;
    let mut tx = fixture.pool.begin().await?;
    assert_eq!(
        claim_open_world_eligibility_data(
            &mut tx,
            &key,
            5999,
            OpenWorldEligibilityData::Respawn { min_s: 5 }
        )
        .await?,
        BossEligibilityClaim::Ineligible
    );
    assert_eq!(
        claim_open_world_eligibility_data(
            &mut tx,
            &key,
            6000,
            OpenWorldEligibilityData::Respawn { min_s: 5 }
        )
        .await?,
        BossEligibilityClaim::Claimed
    );
    // This is a DATA test of supplied reset epochs, never a current-reset entry.
    let at_reset = reward_key("oteryn:boss.reset_boundary");
    for (epoch, want) in [
        (0, BossEligibilityClaim::Claimed),
        (0, BossEligibilityClaim::Ineligible),
        (1, BossEligibilityClaim::Claimed),
        (0, BossEligibilityClaim::Ineligible),
        (u64::MAX, BossEligibilityClaim::Claimed),
        (u64::MAX, BossEligibilityClaim::Ineligible),
    ] {
        assert_eq!(
            claim_open_world_eligibility_data(
                &mut tx,
                &at_reset,
                6000,
                OpenWorldEligibilityData::AtReset { epoch }
            )
            .await?,
            want
        );
    }
    tx.commit().await?;
    let (next, epoch): (i64, Option<String>) = sqlx::query_as(
        "SELECT next_eligible_at,last_credited_epoch::text FROM game_character_boss_open_world_eligibility WHERE boss_key=$1")
        .bind(at_reset.boss.as_str()).fetch_one(&fixture.pool).await?;
    assert_eq!((next, epoch), (0, Some(u64::MAX.to_string())));
    let next: i64 = sqlx::query_scalar(
        "SELECT next_eligible_at FROM game_character_boss_open_world_eligibility WHERE boss_key=$1",
    )
    .bind(key.boss.as_str())
    .fetch_one(&fixture.pool)
    .await?;
    assert_eq!(
        next, 11000,
        "interval starts at the newly credited death, not a bucket boundary"
    );
    assert_eq!(fixture.root_revision().await?, "1");
    Ok(())
}

async fn qualify_boss_cooldowns(fixture: &native_fixture::Harness) -> TestResult {
    // A second genuine Character root, still at bootstrap revision one. It is only
    // a row-lock participant in these PRIVATE DATA tests, never a fabricated session.
    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) \
        SELECT encode($1,'hex')::uuid,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,'Fixture Other' FROM game_character_roots WHERE character_id=encode($2,'hex')::uuid")
        .bind(id(45).as_slice()).bind(id(native_fixture::CHARACTER).as_slice()).execute(&fixture.pool).await?;
    let key = reward_key("oteryn:boss.cooldown");
    let other = BossRewardKey::new(
        CharacterId::from_bytes(id(45)).map_err(native_fixture::debug)?,
        key.boss.as_str(),
    )?;
    let facts = BossCooldownAdmissionData {
        admission: id(90),
        now: 1000,
        reset_epoch: 10,
        cooldown_s: 5,
        boosted: false,
    };
    let mut tx = fixture.pool.begin().await?;
    assert_eq!(
        set_boss_cooldowns_data(&mut tx, std::slice::from_ref(&other), &facts).await?,
        BossCooldownChange::Applied
    );
    tx.commit().await?;
    let later = BossCooldownAdmissionData {
        admission: id(91),
        now: 2000,
        ..facts
    };
    let mut tx = fixture.pool.begin().await?;
    assert_eq!(
        set_boss_cooldowns_data(&mut tx, &[other.clone(), key.clone()], &later).await?,
        BossCooldownChange::OnCooldown
    );
    assert_eq!(
        read_boss_cooldown_data(&mut tx, &key).await?,
        None,
        "preflight prevents a partial group write even if caller commits refusal"
    );
    tx.commit().await?;
    assert_eq!(fixture.count("game_character_boss_cooldowns").await?, 1);
    let mut tx = fixture.pool.begin().await?;
    assert_eq!(
        set_boss_cooldowns_data(&mut tx, std::slice::from_ref(&other), &facts).await?,
        BossCooldownChange::Replayed
    );
    assert!(matches!(
        set_boss_cooldowns_data(&mut tx, &[other.clone(), key.clone()], &facts).await,
        Err(BossRewardStateError::InvalidStoredState)
    ));
    assert_eq!(read_boss_cooldown_data(&mut tx, &key).await?, None);
    let changed = BossCooldownAdmissionData {
        cooldown_s: 6,
        ..facts
    };
    assert!(matches!(
        set_boss_cooldowns_data(&mut tx, std::slice::from_ref(&other), &changed).await,
        Err(BossRewardStateError::ConflictingAdmission)
    ));
    tx.rollback().await?;

    // Exact time boundary and prior-epoch boosted exception are distinct. Testing
    // proposed booleans does not install a producer of current boosted/reset facts.
    for (now, epoch, boosted, want) in [
        (5999, 10, false, BossCooldownChange::OnCooldown),
        (6000, 10, false, BossCooldownChange::Applied),
        (2000, 10, true, BossCooldownChange::OnCooldown),
        (2000, 11, false, BossCooldownChange::OnCooldown),
        (2000, 11, true, BossCooldownChange::Applied),
    ] {
        let mut tx = fixture.pool.begin().await?;
        let change = BossCooldownAdmissionData {
            now,
            reset_epoch: epoch,
            boosted,
            ..later
        };
        assert_eq!(
            set_boss_cooldowns_data(&mut tx, std::slice::from_ref(&other), &change).await?,
            want
        );
        tx.rollback().await?;
    }
    // An expired cooldown is not permission to move its recorded epoch backwards.
    for boosted in [false, true] {
        let mut tx = fixture.pool.begin().await?;
        let stale_epoch = BossCooldownAdmissionData {
            now: 6000,
            reset_epoch: 9,
            boosted,
            ..later
        };
        assert!(matches!(
            set_boss_cooldowns_data(&mut tx, std::slice::from_ref(&other), &stale_epoch).await,
            Err(BossRewardStateError::InvalidInput)
        ));
        tx.rollback().await?;
    }
    let mut tx = fixture.pool.begin().await?;
    assert_eq!(
        compensate_boss_cooldowns_data(&mut tx, &[other.clone(), key.clone()], id(91)).await?,
        0
    );
    assert_eq!(
        compensate_boss_cooldowns_data(&mut tx, std::slice::from_ref(&other), id(90)).await?,
        1
    );
    tx.rollback().await?;
    assert_eq!(
        fixture.count("game_character_boss_cooldowns").await?,
        1,
        "compensation is transactional"
    );
    let replacement = BossCooldownAdmissionData { now: 6000, ..later };
    let mut tx = fixture.pool.begin().await?;
    assert_eq!(
        set_boss_cooldowns_data(&mut tx, &[other.clone(), key.clone()], &replacement).await?,
        BossCooldownChange::Applied
    );
    tx.commit().await?;
    let mut tx = fixture.pool.begin().await?;
    assert_eq!(
        compensate_boss_cooldowns_data(&mut tx, std::slice::from_ref(&other), id(90)).await?,
        0,
        "old abandoned admission cannot refund a newer spent cooldown"
    );
    assert_eq!(
        compensate_boss_cooldowns_data(&mut tx, &[other.clone(), key.clone()], id(91)).await?,
        2
    );
    tx.commit().await?;
    let invalid = BossCooldownAdmissionData {
        cooldown_s: 30 * 24 * 60 * 60 + 1,
        ..replacement
    };
    let mut tx = fixture.pool.begin().await?;
    assert!(matches!(
        set_boss_cooldowns_data(&mut tx, std::slice::from_ref(&key), &invalid).await,
        Err(BossRewardStateError::InvalidInput)
    ));
    assert!(matches!(
        set_boss_cooldowns_data(&mut tx, &[key.clone(), key.clone()], &replacement).await,
        Err(BossRewardStateError::InvalidInput)
    ));
    tx.rollback().await?;
    assert_eq!(fixture.count("game_character_boss_cooldowns").await?, 0);
    assert_eq!(fixture.root_revision().await?, "1");
    Ok(())
}

#[test]
fn boss_reward_native_fences_atomic_claim_and_reload_postgres() -> TestResult {
    let Some(admin) = native_fixture::configured_admin() else {
        return Ok(());
    };
    native_fixture::runtime()?.block_on(async move {
        let fixture = native_fixture::Harness::create(admin, "boss_rewards", false).await?;
        let result = qualify_reward_claims(&fixture).await;
        fixture.cleanup().await?;
        result
    })
}

#[test]
fn boss_reward_time_and_epoch_boundaries_postgres() -> TestResult {
    let Some(admin) = native_fixture::configured_admin() else {
        return Ok(());
    };
    native_fixture::runtime()?.block_on(async move {
        let fixture = native_fixture::Harness::create(admin, "boss_boundaries", false).await?;
        let result = qualify_reward_boundaries(&fixture).await;
        fixture.cleanup().await?;
        result
    })
}

#[test]
fn boss_cooldown_atomic_group_replay_and_compensation_postgres() -> TestResult {
    let Some(admin) = native_fixture::configured_admin() else {
        return Ok(());
    };
    native_fixture::runtime()?.block_on(async move {
        let fixture = native_fixture::Harness::create(admin, "boss_cooldowns", false).await?;
        let result = qualify_boss_cooldowns(&fixture).await;
        fixture.cleanup().await?;
        result
    })
}

#[test]
fn boss_spawn1_activation_reset_and_channel_independence_postgres() -> TestResult {
    use crate::durability::runtime_scope_assignment as native;
    let Some(admin) = native_fixture::configured_admin() else {
        return Ok(());
    };
    native_fixture::runtime()?.block_on(async move {
        let h = native_fixture::Harness::create(admin,"boss_spawn1",false).await?;
        let result: TestResult = async {
            let seal = h.recovery.seal_current().map_err(|e| format!("{e:?}"))?;
            let authority = h.root.open_character_authority(&seal).await.map_err(|e| format!("{e:?}"))?;
            let key = key();
            let respawn = oteryn_gameplay_content::content::BossPolicy::Respawn { min_s:10,max_s:20 };
            use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
            let (controller,pin)=crate::content::native_gameplay::test_support::activated_loop_monster(key.world,false);
            let active=controller.active().ok_or("active content")?;
            let native_content=active.native_gameplay().ok_or("native content")?;
            let make_runtime=|assignment: &native::RuntimeScopeAssignment| -> std::result::Result<crate::foundation::ChannelRuntimeV1,Box<dyn std::error::Error>> {
                let content_pin=crate::content::channel_content_pin::ChannelContentPin::from_activation(
                    key.world,pin.activation_sequence(),active.identity().server_artifact_digest(),active.identity().client_artifact_digest(),
                    pin.frame_binding().digest(),pin.qualified_room().map_revision_digest(),(2,12,-7));
                let mut runtime=crate::foundation::ChannelRuntimeV1::from_committed_assignment(
                    key.world,key.channel,h.node.fact().node_id(),h.node.fact().registration_revision(),
                    assignment.ownership_generation,assignment.source_revision,&assignment.decision_identity,4,content_pin).map_err(|e|format!("{e:?}"))?;
                native_content.install_companion_policies(&mut runtime)?;
                runtime.configure_boss_spawns(vec![crate::foundation::BossSpawn {
                    key:key.spawn.clone(),creature:"oteryn:creature.rat".into(),
                    position:crate::foundation::MovementLocalPosition { x:2,y:12,floor:-7 },
                    policy:respawn,actor:None,cycle:0,pending_claim:false,
                }]).map_err(|e|format!("{e:?}"))?;
                Ok(runtime)
            };

            assert!(matches!(h.root.activate_boss_clock(&authority,&h.node,key.clone(),generation(1),respawn).await?,BossClockClaim::Waiting(_)));
            let mut tx = h.pool.begin().await?;
            let initial = read_clock(&mut tx,&key).await?.ok_or("initial clock")?;
            assert_eq!(initial.state,BossClockState::Due);
            let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&mut *tx).await?;
            assert!((now+9000..=now+20000).contains(&initial.due_at_unix_ms.ok_or("first timer")?));
            tx.rollback().await?;
            sqlx::query("UPDATE game_boss_spawn_clocks SET due_at=0").execute(&h.pool).await?;
            assert!(matches!(h.root.activate_boss_clock(&authority,&h.node,key.clone(),generation(1),respawn).await?,BossClockClaim::Claimed(_)));
            assert_eq!(h.root.activate_boss_clock(&authority,&h.node,key.clone(),generation(1),respawn).await?,BossClockClaim::Unchanged);
            let first_assignment=h.root.read_runtime_scope_assignment(native_fixture::scope()?).await.map_err(|e|format!("{e:?}"))?.ok_or("first assignment")?;
            let mut first_runtime=make_runtime(&first_assignment)?;
            let first_actor=first_runtime.realize_boss_spawn(&key.spawn,0).map_err(|e|format!("{e:?}"))?.ok_or("first actor")?;
            let full=first_runtime.companion_snapshot(first_actor).map_err(|e|format!("{e:?}"))?.maximum_health;
            first_runtime.set_test_creature_health(first_actor,1).map_err(|e|format!("{e:?}"))?;

            // A separately assigned channel gets its own initial timer, independent of ALIVE.
            let mut other = key.clone();other.channel=ChannelId::decode(&id(44))?;
            sqlx::query("INSERT INTO game_control_scope_grants SELECT session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,operation FROM unnest(ARRAY[1,2,3]::smallint[]) AS operation")
                .bind(other.world.as_bytes().as_slice()).bind(other.channel.as_bytes().as_slice()).execute(&h.pool).await?;
            let outcome=h.writer.submit(&native::AssignmentRequest {
                operation_key:native::OperationKey::from_bytes([91;32]),
                actor:native::ControlActor::new("oteryn_test_admin").map_err(|e| format!("{e:?}"))?,
                command:native::AssignmentCommand::Assign {
                    scope:crate::foundation::RuntimeScopeRefV1::channel(other.world,other.channel),
                    target:h.node.fact(),
                },
            }).await.map_err(|e| format!("{e:?}"))?;
            assert!(matches!(outcome,native::AssignmentOutcome::Committed(_)));
            assert!(matches!(h.root.activate_boss_clock(&authority,&h.node,other.clone(),generation(1),respawn).await?,BossClockClaim::Waiting(_)));
            let mut tx=h.pool.begin().await?;
            assert_eq!(read_clock(&mut tx,&key).await?.ok_or("first")?.state,BossClockState::Alive);
            assert_eq!(read_clock(&mut tx,&other).await?.ok_or("second")?.state,BossClockState::Due);
            tx.rollback().await?;
            // The native assignment writer ends generation 1 before recovery can claim ALIVE.
            let scope=native_fixture::scope()?;
            let predecessor=h.root.read_runtime_scope_predecessor(scope).await.map_err(|e| format!("{e:?}"))?.ok_or("predecessor")?;
            h.writer.submit(&native::AssignmentRequest {
                operation_key:native::OperationKey::from_bytes([92;32]),
                actor:native::ControlActor::new("oteryn_test_admin").map_err(|e| format!("{e:?}"))?,
                command:native::AssignmentCommand::Replace { scope,predecessor,target:h.node.fact() },
            }).await.map_err(|e| format!("{e:?}"))?;
            assert!(matches!(h.root.activate_boss_clock(&authority,&h.node,key.clone(),generation(2),respawn).await?,BossClockClaim::Claimed(_)));
            assert_eq!(h.root.activate_boss_clock(&authority,&h.node,key.clone(),generation(2),respawn).await?,BossClockClaim::Unchanged);
            let new_assignment=h.root.read_runtime_scope_assignment(native_fixture::scope()?).await.map_err(|e|format!("{e:?}"))?.ok_or("new assignment")?;
            let mut restarted=make_runtime(&new_assignment)?;
            let recovered=restarted.realize_boss_spawn(&key.spawn,0).map_err(|e|format!("{e:?}"))?.ok_or("recovered actor")?;
            assert_eq!(restarted.companion_snapshot(recovered).map_err(|e|format!("{e:?}"))?.health,full,"ended owner loses fight state and recovers full health");
            assert!(restarted.realize_boss_spawn(&key.spawn,0).is_err(),"one actor in the recovered generation");

            sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_10',1,clock_timestamp(),'2026-10')")
                .bind(key.world.as_bytes().as_slice()).bind([1u8;32].as_slice()).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)").bind(key.world.as_bytes().as_slice()).execute(&h.pool).await?;
            let yes=oteryn_gameplay_content::content::BossPolicy::AtReset { chance:oteryn_gameplay_content::content::BossChance { numerator:1,denominator:1 } };
            let mut reset_key=key.clone();reset_key.spawn="boss_spawn1.reset".into();
            assert!(matches!(h.root.activate_boss_clock(&authority,&h.node,reset_key.clone(),generation(2),yes).await?,BossClockClaim::Claimed(_)));
            let mut tx=h.pool.begin().await?;
            advance_death_in_transaction(&mut tx,&reset_key,generation(2),0,yes).await?;
            tx.commit().await?;
            assert!(matches!(h.root.activate_boss_clock(&authority,&h.node,reset_key.clone(),generation(2),yes).await?,BossClockClaim::Waiting(BossClock { state:BossClockState::Inactive,.. })),"INACTIVE stays inactive in the same epoch");
            sqlx::query("INSERT INTO game_world_reset_epochs SELECT world_id,1,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,clock_timestamp(),reset_month FROM game_world_reset_epochs WHERE reset_epoch=0").execute(&h.pool).await?;
            sqlx::query("UPDATE game_world_reset_active SET reset_epoch=1").execute(&h.pool).await?;
            assert!(matches!(h.root.activate_boss_clock(&authority,&h.node,reset_key.clone(),generation(2),yes).await?,BossClockClaim::Claimed(_)));
            assert_eq!(h.root.activate_boss_clock(&authority,&h.node,reset_key.clone(),generation(2),yes).await?,BossClockClaim::Unchanged,"one draw and installation per epoch");
            let half=oteryn_gameplay_content::content::BossPolicy::AtReset { chance:oteryn_gameplay_content::content::BossChance { numerator:1,denominator:2 } };
            let mut failed=key.clone();
            for ordinal in 0..1000 {
                failed.spawn=format!("boss_spawn1.failed.{ordinal}");
                if oteryn_gameplay_domain::domain::uniform_draw::uniform_draw(draw(&failed,1,"boss_at_reset")?,0,1)==1 { break; }
            }
            let first=h.root.activate_boss_clock(&authority,&h.node,failed.clone(),generation(2),half).await?;
            assert!(matches!(first,BossClockClaim::Waiting(BossClock { state:BossClockState::Inactive,.. })),"a failed nonzero chance draw remains inactive");
            assert_eq!(h.root.activate_boss_clock(&authority,&h.node,failed,generation(2),yes).await?,first,"same epoch is never redrawn, even if content chance changes");
            Ok(())
        }.await;
        h.cleanup().await?;
        result
    })
}
