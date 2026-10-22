//! CHAR-POSITION-1 last-position writer on PostgreSQL 17.6 (decision CHAR-POSITION-0 §3.1-§3.3),
//! with the saved vitals of CHAR-VITALS-1 (migration 0084).
//! The parent harness provides the migrations and an independently seeded current session.
use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, SESSION, TestResult, WORLD, configured_admin, context, debug, fence, id,
    runtime,
};
use crate::domain::progression::{FiniteProgressionPolicy, LevelThreshold};
use crate::durability::DurabilityRoot;
use crate::durability::character_death::{
    CharacterDeathOutcome, CharacterDeathRequest, DeathCell, PlayerDeathOccurrence,
};
use crate::durability::character_last_position::{
    FinalLastPosition, LastPositionCell, LastPositionKey, LastPositionRecord, LastPositionWrite,
    LastVitals, StoredLastPosition,
};
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::foundation::{
    ConnectionGeneration, GameSessionId, RuntimeScopeRefV1, ScopeOwnershipGeneration, WorldId,
};
use oteryn_simulation_determinism::{ExactI64, RoundingMode};

#[path = "tutorial_death_postgres_cases.rs"]
mod tutorial_death;
#[path = "tutorial_islet_placement_postgres_cases.rs"]
mod tutorial_islet_placement;

fn run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, tag, true).await?;
        let outcome = body(&harness).await;
        harness.cleanup().await?;
        outcome
    })
}

fn record(x: i32) -> LastPositionRecord {
    LastPositionRecord {
        world_id: WorldId::decode(&id(WORLD)).expect("World"),
        cell: LastPositionCell { x, y: 6, floor: 7 },
        coordinate_frame_digest: [1; 32],
        map_revision_digest: [2; 32],
        server_artifact_digest: [3; 32],
        vitals: None,
    }
}

/// CHAR-VITALS-1: `record(x)` of a wounded actor.
fn wounded(x: i32, health: u32, mana: u32) -> LastPositionRecord {
    LastPositionRecord {
        vitals: LastVitals::new(health, mana),
        ..record(x)
    }
}

/// The stored row's cell x and vitals as text (`-` for a NULL vital).
async fn vitals_row(harness: &Harness) -> TestResult<Option<String>> {
    Ok(sqlx::query_scalar(
        "SELECT concat_ws('|', tile_x, coalesce(health::text, '-'), coalesce(mana::text, '-')) \
           FROM game_character_last_positions WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(id(CHARACTER).as_slice())
    .fetch_optional(&harness.pool)
    .await?)
}

/// The stored row as text: cell, key and digests' first bytes.
async fn row(harness: &Harness) -> TestResult<Option<String>> {
    Ok(sqlx::query_scalar(
        "SELECT concat_ws('|', world_id::text = $2, tile_x, tile_y, tile_floor, \
                character_lease_generation::text, write_sequence::text, \
                get_byte(coordinate_frame_digest, 0), get_byte(map_revision_digest, 0), \
                get_byte(server_artifact_digest, 0)) \
           FROM game_character_last_positions WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(id(CHARACTER).as_slice())
    .bind(uuid(WORLD))
    .fetch_optional(&harness.pool)
    .await?)
}

async fn write_record(
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    record: LastPositionRecord,
) -> Result<LastPositionWrite, CharacterProgressionError> {
    let fence = fence(1).map_err(|_| CharacterProgressionError::InvalidInput)?;
    h.root
        .write_character_last_position(authority, &h.node, fence, 22, record)
        .await
}

fn uuid(seed: u8) -> String {
    let hex: String = id(seed).iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// §3.1-§3.2: a fenced write stores the cell under its key; a lower key in the same lease never
/// overwrites it; a higher one does; a new node process reads it back for the next admission.
#[test]
fn last_position_orders_by_key_and_survives_a_node_restart() -> TestResult {
    run("last_position_restart", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert_eq!(
            h.root
                .read_character_last_position(&authority, &h.node, fence(1)?)
                .await
                .map_err(debug)?,
            None
        );
        let write = async |sequence: u64, x: i32| -> TestResult<LastPositionWrite> {
            Ok(h.root
                .write_character_last_position(&authority, &h.node, fence(1)?, sequence, record(x))
                .await
                .map_err(debug)?)
        };
        assert_eq!(write(10, 100).await?, LastPositionWrite::Written);
        assert_eq!(row(h).await?.as_deref(), Some("t|100|6|7|1|10|1|2|3"));
        // An in-flight write captured earlier never overwrites a later one.
        assert_eq!(write(9, 101).await?, LastPositionWrite::Superseded);
        assert_eq!(write(10, 102).await?, LastPositionWrite::Superseded);
        assert_eq!(row(h).await?.as_deref(), Some("t|100|6|7|1|10|1|2|3"));
        assert_eq!(write(11, 103).await?, LastPositionWrite::Written);
        // The writes advance no CharacterRevision and leave no receipt.
        assert_eq!(h.root_revision().await?, "1");

        // Restart: a new node process reads the durable row under the same session fence.
        let restarted = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(restarted.maintain_ready_once().await?);
        let restart_seal = h.recovery.seal_current().map_err(debug)?;
        let restart_authority = restarted
            .open_character_authority(&restart_seal)
            .await
            .map_err(debug)?;
        assert_eq!(
            restarted
                .read_character_last_position(&restart_authority, &h.node, fence(1)?)
                .await
                .map_err(debug)?,
            Some(StoredLastPosition {
                record: record(103),
                key: LastPositionKey {
                    character_lease_generation: 1,
                    write_sequence: 11,
                },
            })
        );
        drop(restart_authority);
        drop(restart_seal);
        drop(restarted);
        Ok(())
    })
}

/// §3.2 "Fence": a stale connection, lease, scope generation or terminal session writes and
/// reads nothing; a row of a later lease is never overwritten by an earlier lease's write.
#[test]
fn stale_fences_never_overwrite_the_last_position() -> TestResult {
    run("last_position_stale", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let write = async |fence, sequence: u64, x: i32| {
            h.root
                .write_character_last_position(&authority, &h.node, fence, sequence, record(x))
                .await
        };
        assert!(matches!(
            write(fence(1)?, 20, 200).await,
            Ok(LastPositionWrite::Written)
        ));
        let stored = Some("t|200|6|7|1|20|1|2|3");
        let rejected = |outcome: Result<LastPositionWrite, CharacterProgressionError>| {
            matches!(outcome, Err(CharacterProgressionError::AuthorityRejected))
        };

        let mut connection = fence(1)?;
        connection.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        assert!(rejected(write(connection, 21, 201).await));
        let mut lease = fence(1)?;
        lease.character_lease_generation = 2;
        assert!(rejected(write(lease, 21, 202).await));
        let mut scope = fence(1)?;
        scope.scope_ownership_generation = ScopeOwnershipGeneration::new(2).map_err(debug)?;
        assert!(rejected(write(scope, 21, 203).await));
        let mut session = fence(1)?;
        session.game_session_id = GameSessionId::decode(&id(51)).map_err(debug)?;
        assert!(write(session, 21, 204).await.is_err());
        let mut world = fence(1)?;
        world.runtime_scope = RuntimeScopeRefV1::channel(
            WorldId::decode(&id(44)).map_err(debug)?,
            crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        );
        assert!(write(world, 21, 205).await.is_err());
        assert_eq!(row(h).await?.as_deref(), stored);

        // A record of another World is refused before any fence is read.
        let mut foreign = record(206);
        foreign.world_id = WorldId::decode(&id(44)).map_err(debug)?;
        assert!(matches!(
            write_record(h, &authority, foreign).await,
            Err(CharacterProgressionError::InvalidInput)
        ));

        // A later lease's row stands against the earlier lease, whatever its sequence.
        sqlx::query(
            "UPDATE game_character_last_positions SET character_lease_generation = 2, \
                    write_sequence = 1",
        )
        .execute(&h.pool)
        .await?;
        assert!(matches!(
            write(fence(1)?, u64::MAX, 207).await,
            Ok(LastPositionWrite::Superseded)
        ));
        assert_eq!(row(h).await?.as_deref(), Some("t|200|6|7|2|1|1|2|3"));

        // A terminal session writes and reads nothing.
        sqlx::query(
            "UPDATE game_durability_reconnect_sessions SET session_state = 3 \
              WHERE game_session_id = encode($1,'hex')::uuid",
        )
        .bind(id(SESSION).as_slice())
        .execute(&h.pool)
        .await?;
        assert!(rejected(write(fence(1)?, 30, 208).await));
        assert!(
            h.root
                .read_character_last_position(&authority, &h.node, fence(1)?)
                .await
                .is_err()
        );
        assert_eq!(row(h).await?.as_deref(), Some("t|200|6|7|2|1|1|2|3"));
        Ok(())
    })
}

/// A level-9 Character's death under a ten-level policy (the DEATH-1 writer cases' shape).
fn death(tag: u8) -> TestResult<CharacterDeathRequest<10>> {
    const GLOBAL: [i64; 11] = [0, 100, 200, 400, 800, 1500, 2600, 4200, 6400, 9300, 13000];
    let mut thresholds = [LevelThreshold {
        level: 1,
        minimum_experience: ExactI64::new(0),
    }; 10];
    for (index, threshold) in thresholds.iter_mut().enumerate() {
        *threshold = LevelThreshold {
            level: u32::try_from(index + 1)?,
            minimum_experience: ExactI64::new(GLOBAL[index]),
        };
    }
    Ok(CharacterDeathRequest {
        occurrence: PlayerDeathOccurrence::from_bytes(id(tag)).map_err(debug)?,
        context: context(),
        policy_revision: "policy-1".into(),
        reward_revision: "reward-1".into(),
        policy: FiniteProgressionPolicy {
            context: context(),
            policy_revision: "policy-1".into(),
            reward_revision: "reward-1".into(),
            death_policy_revision: "death-1".into(),
            declared_difference_revision: "declaration-1".into(),
            thresholds,
            terminal_exclusive_experience: ExactI64::new(GLOBAL[10]),
            death_loss_numerator: 1,
            death_loss_denominator: 1,
            death_loss_rounding: RoundingMode::Floor,
        },
        held_blessings: Vec::new(),
        death_cell: DeathCell {
            world_id: WorldId::decode(&id(WORLD)).map_err(debug)?,
            channel_id: crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
            spatial_position: vec![1, 2, 3, 7],
            map_revision: "map-1".into(),
        },
        respawn_position: vec![0, 0, 0, 1, 0, 0, 0, 2, 0, 7],
    })
}

/// §3.2 "Not written" and §3.3 step 1: while a pending respawn exists the position is not
/// written; its consumption writes the placed cell in the same transaction, under the
/// consuming session's lease.
#[test]
fn a_pending_respawn_decides_and_its_consumption_writes_the_placed_cell() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "last_position_respawn", false).await?;
        let outcome = async {
            sqlx::query(
                "INSERT INTO game_character_progression_state VALUES \
                 (encode($1,'hex')::uuid,1,9,6500,'profile-1','ruleset-1','content-1',\
                  'simulation-1','evidence-1','declaration-1','policy-1','reward-1')",
            )
            .bind(id(CHARACTER).as_slice())
            .execute(&h.pool)
            .await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let committed = h
                .root
                .commit_character_death(&authority, &h.node, fence(1)?, death(80)?)
                .await
                .map_err(debug)?;
            assert!(
                matches!(committed, CharacterDeathOutcome::Committed(_)),
                "{committed:?}"
            );
            assert!(matches!(
                h.root
                    .write_character_last_position(&authority, &h.node, fence(2)?, 40, record(300))
                    .await,
                Ok(LastPositionWrite::PendingObligation)
            ));
            assert_eq!(row(&h).await?, None);
            let consumed = h
                .root
                .consume_pending_respawn_placed(
                    &authority,
                    &h.node,
                    fence(2)?,
                    None,
                    FinalLastPosition {
                        record: record(301),
                        write_sequence: 41,
                    },
                )
                .await
                .map_err(debug)?;
            assert!(consumed.is_some());
            assert_eq!(h.count("game_character_pending_respawns").await?, 0);
            assert_eq!(row(&h).await?.as_deref(), Some("t|301|6|7|1|41|1|2|3"));
            drop(authority);
            drop(seal);
            TestResult::Ok(())
        }
        .await;
        h.cleanup().await?;
        outcome
    })
}

/// DEATH-RESTART-SAFETY-1 with §3.2-§3.3 step 1: a live death leaves its respawn pending, so the
/// position written before the death is left and nothing is written over it; the in-session
/// placement then consumes the respawn and upserts the placed cell in the same transaction, so a
/// crash right after the placement never restores the pre-death position. A placed record of
/// another World or with a zero sequence is refused before the placement is called, a refused
/// placement writes nothing, a retry after the consumption writes nothing more, and a capture
/// taken before the placed record never overwrites it.
#[test]
fn an_in_session_respawn_writes_the_placed_cell_with_its_consumption() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "last_position_live_death", false).await?;
        let outcome = async {
            sqlx::query(
                "INSERT INTO game_character_progression_state VALUES \
                 (encode($1,'hex')::uuid,1,9,6500,'profile-1','ruleset-1','content-1',\
                  'simulation-1','evidence-1','declaration-1','policy-1','reward-1')",
            )
            .bind(id(CHARACTER).as_slice())
            .execute(&h.pool)
            .await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            // The living actor's last periodic write, before the death.
            assert!(matches!(
                h.root
                    .write_character_last_position(
                        &authority,
                        &h.node,
                        fence(1)?,
                        50,
                        wounded(400, 9, 2)
                    )
                    .await,
                Ok(LastPositionWrite::Written)
            ));
            assert_eq!(vitals_row(&h).await?.as_deref(), Some("400|9|2"));
            let sequencer = CharacterRevisionSequencer::new();
            h.root
                .settle_player_death(&sequencer, &authority, &h.node, fence(1)?, death(81)?)
                .await
                .map_err(debug)?;
            // The death leaves its respawn recorded and the position untouched.
            assert_eq!(h.count("game_character_pending_respawns").await?, 1);
            assert_eq!(row(&h).await?.as_deref(), Some("t|400|6|7|1|50|1|2|3"));
            let occurrence = PlayerDeathOccurrence::from_bytes(id(81)).map_err(debug)?;
            let placed = FinalLastPosition {
                record: record(401),
                write_sequence: 51,
            };
            // A placed record of another World, or with a zero sequence, is refused before the
            // placement is called.
            let mut foreign = placed;
            foreign.record.world_id = WorldId::decode(&id(44)).map_err(debug)?;
            let mut unordered = placed;
            unordered.write_sequence = 0;
            let called = std::sync::atomic::AtomicBool::new(false);
            for refused in [foreign, unordered] {
                assert!(matches!(
                    h.root
                        .place_and_consume_pending_respawn_placed(
                            &authority,
                            &h.node,
                            fence(2)?,
                            occurrence,
                            refused,
                            |_| {
                                called.store(true, std::sync::atomic::Ordering::Relaxed);
                                async { true }
                            },
                        )
                        .await,
                    Err(CharacterProgressionError::InvalidInput)
                ));
            }
            assert!(!called.load(std::sync::atomic::Ordering::Relaxed));
            // A refused placement consumes nothing and writes nothing.
            assert!(
                !h.root
                    .place_and_consume_pending_respawn_placed(
                        &authority,
                        &h.node,
                        fence(2)?,
                        occurrence,
                        placed,
                        |_| async { false },
                    )
                    .await
                    .map_err(debug)?
            );
            assert_eq!(h.count("game_character_pending_respawns").await?, 1);
            assert_eq!(row(&h).await?.as_deref(), Some("t|400|6|7|1|50|1|2|3"));
            // The placement consumes the respawn and upserts the placed cell with it.
            assert!(
                h.root
                    .place_and_consume_pending_respawn_placed(
                        &authority,
                        &h.node,
                        fence(2)?,
                        occurrence,
                        placed,
                        |pending| async move { pending.is_some() },
                    )
                    .await
                    .map_err(debug)?
            );
            assert_eq!(h.count("game_character_pending_respawns").await?, 0);
            assert_eq!(row(&h).await?.as_deref(), Some("t|401|6|7|1|51|1|2|3"));
            // CHAR-VITALS-1: the respawned actor is at its maxima, so the placement clears the
            // vitals the pre-death write saved; a later login never restores a dying actor's.
            assert_eq!(vitals_row(&h).await?.as_deref(), Some("401|-|-"));
            // A retry after the consumption is handed no row and writes nothing more.
            assert!(
                h.root
                    .place_and_consume_pending_respawn_placed(
                        &authority,
                        &h.node,
                        fence(2)?,
                        occurrence,
                        FinalLastPosition {
                            record: record(402),
                            write_sequence: 52,
                        },
                        |pending| async move { pending.is_none() },
                    )
                    .await
                    .map_err(debug)?
            );
            assert_eq!(row(&h).await?.as_deref(), Some("t|401|6|7|1|51|1|2|3"));
            // A capture taken before the placed record never overwrites it, nor its vitals.
            assert!(matches!(
                h.root
                    .write_character_last_position(
                        &authority,
                        &h.node,
                        fence(2)?,
                        50,
                        wounded(403, 1, 0)
                    )
                    .await,
                Ok(LastPositionWrite::Superseded)
            ));
            assert_eq!(row(&h).await?.as_deref(), Some("t|401|6|7|1|51|1|2|3"));
            assert_eq!(vitals_row(&h).await?.as_deref(), Some("401|-|-"));
            drop(authority);
            drop(seal);
            TestResult::Ok(())
        }
        .await;
        h.cleanup().await?;
        outcome
    })
}

/// CHAR-VITALS-1: the vitals ride the cell's upsert, so they are ordered and fenced with it: a
/// lower or equal key and every stale fence leave the stored vitals; a greater key replaces the
/// cell and the vitals together, a row without vitals clears them; a new node process reads them
/// back. No write advances the CharacterRevision.
#[test]
fn saved_vitals_are_ordered_and_fenced_with_the_cell_and_survive_a_restart() -> TestResult {
    run("last_position_vitals", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let write = async |fence, sequence: u64, record: LastPositionRecord| {
            h.root
                .write_character_last_position(&authority, &h.node, fence, sequence, record)
                .await
        };
        assert!(matches!(
            write(fence(1)?, 10, wounded(500, 40, 3)).await,
            Ok(LastPositionWrite::Written)
        ));
        assert_eq!(vitals_row(h).await?.as_deref(), Some("500|40|3"));
        // A capture taken earlier never overwrites the vitals, even with the same cell.
        for sequence in [9, 10] {
            assert!(matches!(
                write(fence(1)?, sequence, wounded(500, 1, 1)).await,
                Ok(LastPositionWrite::Superseded)
            ));
        }
        // Every stale fence writes neither the cell nor the vitals.
        let mut connection = fence(1)?;
        connection.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        let mut lease = fence(1)?;
        lease.character_lease_generation = 2;
        let mut scope = fence(1)?;
        scope.scope_ownership_generation = ScopeOwnershipGeneration::new(2).map_err(debug)?;
        for stale in [connection, lease, scope] {
            assert!(matches!(
                write(stale, 11, wounded(500, 2, 2)).await,
                Err(CharacterProgressionError::AuthorityRejected)
            ));
        }
        let mut session = fence(1)?;
        session.game_session_id = GameSessionId::decode(&id(51)).map_err(debug)?;
        assert!(write(session, 11, wounded(500, 2, 2)).await.is_err());
        assert_eq!(vitals_row(h).await?.as_deref(), Some("500|40|3"));
        // A later capture of the same cell with other vitals is a write of its own.
        assert!(matches!(
            write(fence(1)?, 11, wounded(500, 41, 0)).await,
            Ok(LastPositionWrite::Written)
        ));
        assert_eq!(vitals_row(h).await?.as_deref(), Some("500|41|0"));
        // An actor saved at its maxima stores none.
        assert!(matches!(
            write(fence(1)?, 12, record(501)).await,
            Ok(LastPositionWrite::Written)
        ));
        assert_eq!(vitals_row(h).await?.as_deref(), Some("501|-|-"));
        assert!(matches!(
            write(fence(1)?, 13, wounded(502, 7, 5)).await,
            Ok(LastPositionWrite::Written)
        ));
        // The writes advance no CharacterRevision and leave no receipt.
        assert_eq!(h.root_revision().await?, "1");

        // Restart: a new node process reads the vitals back with the cell.
        let restarted = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(restarted.maintain_ready_once().await?);
        let restart_seal = h.recovery.seal_current().map_err(debug)?;
        let restart_authority = restarted
            .open_character_authority(&restart_seal)
            .await
            .map_err(debug)?;
        assert_eq!(
            restarted
                .read_character_last_position(&restart_authority, &h.node, fence(1)?)
                .await
                .map_err(debug)?,
            Some(StoredLastPosition {
                record: wounded(502, 7, 5),
                key: LastPositionKey {
                    character_lease_generation: 1,
                    write_sequence: 13,
                },
            })
        );
        drop(restart_authority);
        drop(restart_seal);
        drop(restarted);
        Ok(())
    })
}

/// §3.1: the World guard, the order trigger and the grants hold for every writer.
#[test]
fn the_table_refuses_foreign_worlds_lower_keys_and_deletes() -> TestResult {
    run("last_position_guards", async |h| {
        let insert = |world: u8, lease: i64| {
            sqlx::query(
                "INSERT INTO game_character_last_positions VALUES \
                 (encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, 2, 7, \
                  decode(repeat('01', 32), 'hex'), decode(repeat('02', 32), 'hex'), \
                  decode(repeat('03', 32), 'hex'), $3, 5)",
            )
            .bind(id(CHARACTER).as_slice())
            .bind(id(world).as_slice())
            .bind(lease)
            .execute(&h.pool)
        };
        assert!(insert(44, 1).await.is_err(), "foreign World");
        insert(WORLD, 1).await?;
        for update in [
            "UPDATE game_character_last_positions SET write_sequence = 4",
            "UPDATE game_character_last_positions SET tile_x = 9",
            "UPDATE game_character_last_positions SET world_id = '019a0000-0000-7000-8000-000000000044', write_sequence = 6",
        ] {
            assert!(
                sqlx::query(update).execute(&h.pool).await.is_err(),
                "{update}"
            );
        }
        sqlx::query("UPDATE game_character_last_positions SET tile_x = 9, write_sequence = 6")
            .execute(&h.pool)
            .await?;
        // CHAR-VITALS-1 (migration 0084): both vitals or neither, a living health and both
        // within the wire bound, whatever the key; a row of migration 0080 holds none.
        assert!(
            sqlx::query_scalar::<_, bool>(
                "SELECT health IS NULL AND mana IS NULL FROM game_character_last_positions"
            )
            .fetch_one(&h.pool)
            .await?
        );
        for update in [
            "UPDATE game_character_last_positions SET health = 5, mana = NULL, write_sequence = 7",
            "UPDATE game_character_last_positions SET health = NULL, mana = 5, write_sequence = 7",
            "UPDATE game_character_last_positions SET health = 0, mana = 5, write_sequence = 7",
            "UPDATE game_character_last_positions SET health = -1, mana = 5, write_sequence = 7",
            "UPDATE game_character_last_positions SET health = 5, mana = -1, write_sequence = 7",
            "UPDATE game_character_last_positions SET health = 268435456, mana = 5, \
                    write_sequence = 7",
            "UPDATE game_character_last_positions SET health = 5, mana = 268435456, \
                    write_sequence = 7",
        ] {
            assert!(
                sqlx::query(update).execute(&h.pool).await.is_err(),
                "{update}"
            );
        }
        // Vitals change only with a raised key, as the cell does.
        assert!(
            sqlx::query("UPDATE game_character_last_positions SET health = 5, mana = 0")
                .execute(&h.pool)
                .await
                .is_err()
        );
        sqlx::query(
            "UPDATE game_character_last_positions SET health = 268435455, mana = 268435455, \
                    write_sequence = 7",
        )
        .execute(&h.pool)
        .await?;
        assert!(
            sqlx::query("TRUNCATE game_character_last_positions")
                .execute(&h.pool)
                .await
                .is_err()
        );
        let grants: (bool, bool, bool, bool, bool, bool) = sqlx::query_as(
            "SELECT has_table_privilege('oteryn_game_runtime', 'game_character_last_positions', 'SELECT'), \
                    has_table_privilege('oteryn_game_runtime', 'game_character_last_positions', 'INSERT'), \
                    has_table_privilege('oteryn_game_runtime', 'game_character_last_positions', 'UPDATE'), \
                    has_table_privilege('oteryn_game_runtime', 'game_character_last_positions', 'DELETE'), \
                    has_table_privilege('oteryn_game_control', 'game_character_last_positions', 'SELECT'), \
                    has_table_privilege('oteryn_game_control', 'game_character_last_positions', 'UPDATE')",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(grants, (true, true, true, false, true, false));
        Ok(())
    })
}
