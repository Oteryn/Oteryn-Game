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
const CLOCK_SCHEMA: &str = r#"
CREATE TABLE game_boss_spawn_clocks (
 world_id UUID NOT NULL,channel_id UUID NOT NULL,spawn_key TEXT NOT NULL CHECK(octet_length(spawn_key)>0),
 cycle NUMERIC(20,0) NOT NULL CHECK(cycle BETWEEN 0 AND 18446744073709551615),
 state SMALLINT NOT NULL CHECK(state IN (1,2,3)),
 reset_epoch NUMERIC(20,0) NOT NULL CHECK(reset_epoch BETWEEN 0 AND 18446744073709551615),
 due_at BIGINT NULL CHECK(due_at>=0),
 owner_generation NUMERIC(20,0) NULL CHECK(owner_generation BETWEEN 1 AND 18446744073709551615),
 PRIMARY KEY(world_id,channel_id,spawn_key),
 CHECK((state=1 AND due_at IS NOT NULL AND owner_generation IS NULL)
    OR (state=2 AND due_at IS NULL AND owner_generation IS NOT NULL)
    OR (state=3 AND due_at IS NULL AND owner_generation IS NULL))
);
"#;

use crate::bestiary_postgres_harness as native_fixture;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

async fn qualify_clock_storage(fixture: &native_fixture::Harness) -> TestResult {
    use super::super::runtime_scope_assignment::{BootstrapSecret, NodeRegistrationFact};
    use crate::durability::runtime_scope_assignment as native;
    use sqlx::{Connection, Executor};
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
    pool.execute(sqlx::raw_sql(CLOCK_SCHEMA)).await?;
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
        eprintln!("Canonical PostgreSQL17.6; genuine native Root/node/assignment/recovery fixture");
        let result = qualify_clock_storage(&fixture).await;
        fixture.cleanup().await?;
        result
    })
}
