// Test-only native PG exercise. Registered by the canonical PG root, never by lib tests.
const TEST_SCHEMA: &str = "CREATE TABLE game_character_pending_arrivals (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    occurrence_id UUID NOT NULL, world_id UUID NOT NULL,
    tile_x INTEGER NOT NULL, tile_y INTEGER NOT NULL, tile_floor SMALLINT NOT NULL
)";

use super::*;
use crate::bestiary_postgres_harness::{self as fixture, TestResult, debug, fence, id};
use sqlx::Connection;
fn occurrence(seed: u8) -> Result<TravelOccurrence, String> {
    TravelOccurrence::from_bytes(id(seed)).map_err(debug)
}
fn placed(sequence: u64, x: i32) -> TestResult<FinalLastPosition> {
    Ok(FinalLastPosition {
        write_sequence: sequence,
        record: super::super::character_last_position::LastPositionRecord {
            world_id: WorldId::decode(&id(fixture::WORLD)).map_err(debug)?,
            cell: LastPositionCell { x, y: 20, floor: 7 },
            coordinate_frame_digest: [1; 32],
            map_revision_digest: [2; 32],
            server_artifact_digest: [3; 32],
            vitals: None,
        },
    })
}
async fn seed(h: &fixture::Harness, seed: u8) -> TestResult {
    sqlx::query("INSERT INTO game_character_pending_arrivals VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, 10,20,7)")
    .bind(id(fixture::CHARACTER).as_slice()).bind(id(seed).as_slice()).bind(id(fixture::WORLD).as_slice()).execute(&h.pool).await?;
    Ok(())
}
async fn row(h: &fixture::Harness) -> TestResult<Option<String>> {
    Ok(sqlx::query_scalar("SELECT occurrence_id::text FROM game_character_pending_arrivals WHERE character_id=encode($1,'hex')::uuid")
    .bind(id(fixture::CHARACTER).as_slice()).fetch_optional(&h.pool).await?)
}
async fn position(h: &fixture::Harness) -> TestResult<Option<String>> {
    Ok(sqlx::query_scalar("SELECT concat_ws('|',tile_x,character_lease_generation::text,write_sequence::text) FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid")
    .bind(id(fixture::CHARACTER).as_slice()).fetch_optional(&h.pool).await?)
}

pub(crate) fn npc_travel_native_postgres_replay_fences_rollback_and_reload() -> TestResult {
    let admin = fixture::configured_admin().ok_or("canonical PostgreSQL17.6 must be configured")?;
    fixture::runtime()?.block_on(async {
        let h = fixture::Harness::create(admin, "npc_travel", true).await?;
        let result = exercise(&h).await;
        h.cleanup().await?;
        result
    })
}

async fn exercise(h: &fixture::Harness) -> TestResult {
    sqlx::query(TEST_SCHEMA).execute(&h.pool).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let current = fence(1)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    assert_eq!(
        h.root
            .read_npc_pending_arrival(&authority, &h.node, current)
            .await
            .map_err(debug)?,
        None
    );
    let mut tx = h.pool.begin().await?;
    assert_eq!(
        read_fenced(&mut tx, seal.record(), &h.node, current)
            .await
            .map_err(debug)?,
        None
    );
    tx.rollback().await?;
    seed(h, 70).await?; // Fixture only: no fee/atomic travel producer is claimed.
    let original = row(h).await?;
    let mut negatives = Vec::new();
    let mut wrong = current;
    wrong.character_id = CharacterId::from_bytes(id(99)).map_err(debug)?;
    negatives.push(wrong);
    let mut wrong = current;
    wrong.connection_generation = crate::foundation::ConnectionGeneration::new(2).map_err(debug)?;
    negatives.push(wrong);
    let mut wrong = current;
    wrong.character_lease_generation = 2;
    negatives.push(wrong);
    let mut wrong = current;
    wrong.scope_ownership_generation =
        crate::foundation::ScopeOwnershipGeneration::new(2).map_err(debug)?;
    negatives.push(wrong);
    let mut wrong = current;
    wrong.runtime_scope = RuntimeScopeRefV1::channel(
        WorldId::decode(&id(99)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(fixture::CHANNEL)).map_err(debug)?,
    );
    negatives.push(wrong);
    let mut wrong = current;
    wrong.runtime_scope = RuntimeScopeRefV1::channel(
        WorldId::decode(&id(fixture::WORLD)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(99)).map_err(debug)?,
    );
    negatives.push(wrong);
    for wrong in negatives {
        let mut tx = h.pool.begin().await?;
        assert!(matches!(
            consume_in_transaction(
                &mut tx,
                seal.record(),
                &h.node,
                wrong,
                occurrence(70)?,
                placed(10, 100)?
            )
            .await,
            Err(TravelStoreError::Fence(
                CharacterProgressionError::AuthorityRejected
            )) | Err(TravelStoreError::InvalidInput)
        ));
        tx.rollback().await?;
        assert_eq!(row(h).await?, original);
        assert_eq!(position(h).await?, None);
    }
    let mut wrong_recovery = seal.record().clone();
    wrong_recovery.authority_scope_id.push_str("-other");
    let mut tx = h.pool.begin().await?;
    assert!(matches!(
        read_fenced(&mut tx, &wrong_recovery, &h.node, current).await,
        Err(TravelStoreError::Storage(_))
    ));
    tx.rollback().await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("UPDATE game_runtime_scope_assignment_writer SET source_revision_high_water=source_revision_high_water+1 WHERE writer_id=1")
    .execute(&mut *tx)
    .await?;
    assert!(matches!(
        read_fenced(&mut tx, seal.record(), &h.node, current).await,
        Err(TravelStoreError::Integrity)
    ));
    tx.rollback().await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("UPDATE game_character_pending_arrivals SET world_id=encode($1,'hex')::uuid")
        .bind(id(99).as_slice())
        .execute(&mut *tx)
        .await?;
    assert!(matches!(
        read_fenced(&mut tx, seal.record(), &h.node, current).await,
        Err(TravelStoreError::Integrity)
    ));
    tx.rollback().await?;
    let mut tx = h.pool.begin().await?;
    // A malformed simultaneous obligation is an integrity refusal, never a placement choice.
    // Its native deferred receipt FK is never committed by this invalid-state fixture.
    sqlx::query("INSERT INTO game_character_pending_respawns VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3)")
    .bind(id(fixture::CHARACTER).as_slice()).bind(id(90).as_slice()).bind(vec![1_u8]).execute(&mut *tx).await?;
    assert!(matches!(
        consume_in_transaction(
            &mut tx,
            seal.record(),
            &h.node,
            current,
            occurrence(70)?,
            placed(10, 100)?
        )
        .await,
        Err(TravelStoreError::Integrity)
    ));
    tx.rollback().await?;
    assert_eq!(row(h).await?, original);
    assert_eq!(position(h).await?, None);
    let mut tx = h.pool.begin().await?;
    let fact = h.node.fact();
    let wrong_secret = NodeIncarnationProof::new(
        fact,
        crate::durability::runtime_scope_assignment::BootstrapSecret::from_bytes([99; 32]),
    );
    assert!(matches!(
        read_fenced(&mut tx, seal.record(), &wrong_secret, current).await,
        Err(TravelStoreError::Fence(_))
    ));
    tx.rollback().await?;
    let mut tx = h.pool.begin().await?;
    assert_eq!(
        consume_in_transaction(
            &mut tx,
            seal.record(),
            &h.node,
            current,
            occurrence(71)?,
            placed(10, 100)?
        )
        .await
        .map_err(debug)?,
        ArrivalConsume::NoOp
    );
    tx.commit().await?;
    assert_eq!(row(h).await?, original);
    assert_eq!(position(h).await?, None);
    // A pending change is not a commit: both delete and position upsert roll back together.
    let mut tx = h.pool.begin().await?;
    assert_eq!(
        consume_in_transaction(
            &mut tx,
            seal.record(),
            &h.node,
            current,
            occurrence(70)?,
            placed(10, 100)?
        )
        .await
        .map_err(debug)?,
        ArrivalConsume::Consumed
    );
    tx.rollback().await?;
    assert_eq!(row(h).await?, original);
    assert_eq!(position(h).await?, None);
    let mut tx = h.pool.begin().await?;
    assert_eq!(
        consume_in_transaction(
            &mut tx,
            seal.record(),
            &h.node,
            current,
            occurrence(70)?,
            placed(10, 100)?
        )
        .await
        .map_err(debug)?,
        ArrivalConsume::Consumed
    );
    tx.commit().await?;
    assert_eq!(row(h).await?, None);
    assert_eq!(position(h).await?.as_deref(), Some("100|1|10"));
    let mut tx = h.pool.begin().await?;
    assert_eq!(
        consume_in_transaction(
            &mut tx,
            seal.record(),
            &h.node,
            current,
            occurrence(70)?,
            placed(11, 999)?
        )
        .await
        .map_err(debug)?,
        ArrivalConsume::NoOp
    );
    tx.commit().await?;
    assert_eq!(position(h).await?.as_deref(), Some("100|1|10"));
    seed(h, 71).await?;
    let new_row = row(h).await?;
    let mut tx = h.pool.begin().await?;
    assert_eq!(
        consume_in_transaction(
            &mut tx,
            seal.record(),
            &h.node,
            current,
            occurrence(70)?,
            placed(11, 999)?
        )
        .await
        .map_err(debug)?,
        ArrivalConsume::NoOp
    );
    tx.commit().await?;
    assert_eq!(row(h).await?, new_row);
    let mut tx = h.pool.begin().await?;
    assert!(matches!(
        consume_in_transaction(
            &mut tx,
            seal.record(),
            &h.node,
            current,
            occurrence(71)?,
            placed(9, 999)?
        )
        .await,
        Err(TravelStoreError::PositionNotWritten(
            LastPositionWrite::Superseded
        ))
    ));
    tx.rollback().await?;
    assert_eq!(row(h).await?, new_row);
    assert_eq!(position(h).await?.as_deref(), Some("100|1|10"));
    let mut tx = h.pool.begin().await?;
    assert_eq!(
        consume_in_transaction(
            &mut tx,
            seal.record(),
            &h.node,
            current,
            occurrence(71)?,
            placed(11, 200)?
        )
        .await
        .map_err(debug)?,
        ArrivalConsume::Consumed
    );
    tx.commit().await?;
    assert_eq!(h.root_revision().await?, "1");
    seed(h, 72).await?;
    let spawn_consume = || {
        let pool = h.pool.clone();
        let recovery = seal.record().clone();
        let node = h.node.clone();
        tokio::spawn(async move {
            let mut tx = pool.begin().await.map_err(debug)?;
            let result = consume_in_transaction(
                &mut tx,
                &recovery,
                &node,
                current,
                occurrence(72)?,
                placed(12, 300).map_err(debug)?,
            )
            .await
            .map_err(debug)?;
            tx.commit().await.map_err(debug)?;
            Ok::<_, String>(result)
        })
    };
    let first = spawn_consume();
    let second = spawn_consume();
    let outcomes = [first.await??, second.await??];
    assert_eq!(
        outcomes
            .iter()
            .filter(|o| **o == ArrivalConsume::Consumed)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|o| **o == ArrivalConsume::NoOp)
            .count(),
        1
    );
    assert_eq!(row(h).await?, None);
    let mut reopened = sqlx::PgConnection::connect(&h.database.url).await?;
    let cell: i32 = sqlx::query_scalar("SELECT tile_x FROM game_character_last_positions")
        .fetch_one(&mut reopened)
        .await?;
    assert_eq!(cell, 300);
    reopened.close().await?;
    seed(h, 73).await?;
    assert_eq!(
        h.root
            .consume_npc_pending_arrival_placed(
                &authority,
                &h.node,
                current,
                occurrence(73)?,
                placed(13, 400)?
            )
            .await
            .map_err(debug)?,
        ArrivalConsume::Consumed
    );
    assert_eq!(
        h.root
            .consume_npc_pending_arrival_placed(
                &authority,
                &h.node,
                current,
                occurrence(73)?,
                placed(14, 999)?
            )
            .await
            .map_err(debug)?,
        ArrivalConsume::NoOp
    );
    assert_eq!(position(h).await?.as_deref(), Some("400|1|13"));
    // Exercise the real semantic wrapper's typed-refusal rollback too.
    seed(h, 74).await?;
    let before = row(h).await?;
    assert!(matches!(
        h.root
            .consume_npc_pending_arrival_placed(
                &authority,
                &h.node,
                current,
                occurrence(74)?,
                placed(12, 999)?
            )
            .await,
        Err(TravelStoreError::PositionNotWritten(
            LastPositionWrite::Superseded
        ))
    ));
    assert_eq!(row(h).await?, before);
    assert_eq!(position(h).await?.as_deref(), Some("400|1|13"));
    h.root
        .revoke_node_registration(h.node.fact())
        .await
        .map_err(debug)?;
    assert!(matches!(
        h.root
            .read_npc_pending_arrival(&authority, &h.node, current)
            .await,
        Err(TravelStoreError::Fence(
            CharacterProgressionError::AuthorityRejected
        ))
    ));
    assert_eq!(row(h).await?, before);
    println!(
        "Canonical PostgreSQL17.6; genuine native recovery/session/owner APIs; exact occurrence consume/replay/rollback/reload PASS; no physical placement or fee writer claimed"
    );
    Ok(())
}
