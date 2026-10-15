//! TRAVEL0 §7.4 private pending-arrival reader and occurrence-keyed consume.
//!
//! No fee/arrival producer exists here: a travel cause, its fee and this obligation must
//! originate in ONE item-only transaction. The current CharmUnassign fee writer cannot do that.
//! `FinalLastPosition` is a capture, never proof of relocation. The caller must have actually
//! placed the living, uncancelled actor under all TRAVEL0 §7.3 owner fences before consuming.
//! Registration, production DDL/roles, death deletion, admission and the periodic/final
//! position writers' pending-arrival guard remain mandatory integration work.

#![allow(
    dead_code,
    reason = "Private accepted travel component; production owner integration remains a CP slot"
)]

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence, uuid_text};
use super::character_last_position::{
    FinalLastPosition, LastPositionCell, LastPositionKey, LastPositionWrite, upsert_last_position,
};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_session_fence,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_mint_audit::check_uuid_v7;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use crate::domain::CharacterId;
use crate::foundation::{RuntimeScopeRefV1, WorldId};
use sqlx::Row;

/// The confirming command's occurrence. Construction checks representation only; the
/// eventual cause writer must enforce its one-to-one CommandRef binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TravelOccurrence([u8; 16]);

impl TravelOccurrence {
    pub(crate) fn from_bytes(bytes: [u8; 16]) -> Result<Self, TravelStoreError> {
        check_uuid_v7(&bytes).map_err(|_| TravelStoreError::InvalidInput)?;
        Ok(Self(bytes))
    }

    pub(crate) const fn as_bytes(self) -> [u8; 16] {
        self.0
    }
}

/// An obligation carries no Channel: a later admission chooses its Channel normally (§8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PendingArrival {
    pub(crate) occurrence: TravelOccurrence,
    pub(crate) world_id: WorldId,
    pub(crate) destination: LastPositionCell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArrivalConsume {
    Consumed,
    /// Already consumed, deleted by death, or another occurrence is pending. No position write.
    NoOp,
}

/// Read-only consume reconciliation. Absence never proves the fee aborted: only the
/// retained travel cause/command outcome can reconcile the initial fee transaction (§7.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConsumeReplay {
    RetryConsume,
    Complete,
}

pub(crate) fn classify_consume(
    stored: Option<PendingArrival>,
    occurrence: TravelOccurrence,
) -> ConsumeReplay {
    if stored.is_some_and(|row| row.occurrence == occurrence) {
        ConsumeReplay::RetryConsume
    } else {
        ConsumeReplay::Complete
    }
}

#[derive(Debug)]
pub(crate) enum TravelStoreError {
    InvalidInput,
    AuthorityRejected,
    Fence(CharacterProgressionError),
    Integrity,
    PositionNotWritten(LastPositionWrite),
    Storage(DurabilityError),
}

impl From<DurabilityError> for TravelStoreError {
    fn from(error: DurabilityError) -> Self {
        Self::Storage(error)
    }
}
impl From<sqlx::Error> for TravelStoreError {
    fn from(error: sqlx::Error) -> Self {
        Self::Storage(error.into())
    }
}

impl DurabilityRoot {
    pub(crate) async fn read_npc_pending_arrival(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<Option<PendingArrival>, TravelStoreError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| TravelStoreError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = read_fenced(&mut tx, &recovery, &node, fence).await;
                    if result.is_ok() {
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(result)
                })
            })
            .await?
    }

    /// The actual placement/capture precedes this call; retain the runtime hold until its
    /// commit is known, or reconcile by an independently session-fenced read (§7.4).
    pub(crate) async fn consume_npc_pending_arrival_placed(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        occurrence: TravelOccurrence,
        placed: FinalLastPosition,
    ) -> Result<ArrivalConsume, TravelStoreError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| TravelStoreError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = consume_in_transaction(
                        &mut tx, &recovery, &node, fence, occurrence, placed,
                    )
                    .await;
                    // In particular a superseded/pending-obligation position write MUST roll back
                    // the preceding delete. Nothing is committed on any typed refusal.
                    if result.is_ok() {
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(result)
                })
            })
            .await?
    }
}

async fn read_fenced(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    recovery: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
) -> Result<Option<PendingArrival>, TravelStoreError> {
    let RuntimeScopeRefV1::Channel { world_id, .. } = fence.runtime_scope else {
        return Err(TravelStoreError::InvalidInput);
    };
    if fence.character_lease_generation == 0 {
        return Err(TravelStoreError::InvalidInput);
    }
    assert_recovery_fence(tx, recovery).await?;
    lock_admission_relations(tx).await?;
    let history_valid: bool =
        sqlx::query_scalar("SELECT game_runtime_scope_assignment_history_valid()")
            .fetch_one(&mut **tx)
            .await?;
    if !history_valid {
        return Err(TravelStoreError::Integrity);
    }
    assert_session_fence(tx, &fence.item_fence(), node)
        .await?
        .map_err(TravelStoreError::Fence)?;
    let row = load_arrival(tx, fence.character_id).await?;
    if let Some(row) = row {
        if row.world_id != world_id {
            return Err(TravelStoreError::Integrity);
        }
        let respawn: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM game_character_pending_respawns WHERE character_id = encode($1,'hex')::uuid)"
        ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
        if respawn {
            return Err(TravelStoreError::Integrity);
        }
    }
    Ok(row)
}

/// Caller holds the native reconciled authority/recovery seal for this entire transaction.
/// On ANY error it rolls back the whole transaction. No standalone producer is exposed.
async fn consume_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    recovery: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    occurrence: TravelOccurrence,
    placed: FinalLastPosition,
) -> Result<ArrivalConsume, TravelStoreError> {
    if placed.write_sequence == 0
        || !matches!(fence.runtime_scope,
        RuntimeScopeRefV1::Channel { world_id, .. } if world_id == placed.record.world_id)
    {
        return Err(TravelStoreError::InvalidInput);
    }
    let row = read_fenced(tx, recovery, node, fence).await?;
    if classify_consume(row, occurrence) == ConsumeReplay::Complete {
        return Ok(ArrivalConsume::NoOp);
    }
    let deleted = sqlx::query(
        "DELETE FROM game_character_pending_arrivals WHERE character_id = encode($1,'hex')::uuid AND occurrence_id = encode($2,'hex')::uuid"
    ).bind(fence.character_id.as_bytes().as_slice()).bind(occurrence.0.as_slice())
        .execute(&mut **tx).await?;
    if deleted.rows_affected() != 1 {
        return Err(TravelStoreError::Integrity);
    }
    let written = upsert_last_position(
        tx,
        fence.character_id.as_bytes(),
        LastPositionKey {
            character_lease_generation: fence.character_lease_generation,
            write_sequence: placed.write_sequence,
        },
        &placed.record,
    )
    .await?;
    if written != LastPositionWrite::Written {
        return Err(TravelStoreError::PositionNotWritten(written));
    }
    Ok(ArrivalConsume::Consumed)
}

async fn load_arrival(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
) -> Result<Option<PendingArrival>, TravelStoreError> {
    let row = sqlx::query(
        "SELECT occurrence_id::text, world_id::text, tile_x, tile_y, tile_floor FROM game_character_pending_arrivals WHERE character_id = encode($1,'hex')::uuid FOR UPDATE"
    ).bind(character.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    row.map(|row| {
        Ok(PendingArrival {
            occurrence: TravelOccurrence::from_bytes(uuid_text(row.try_get("occurrence_id")?)?)
                .map_err(|_| TravelStoreError::Integrity)?,
            world_id: WorldId::decode(&uuid_text(row.try_get("world_id")?)?)
                .map_err(|_| TravelStoreError::Integrity)?,
            destination: LastPositionCell {
                x: row.try_get("tile_x")?,
                y: row.try_get("tile_y")?,
                floor: row.try_get("tile_floor")?,
            },
        })
    })
    .transpose()
}

#[cfg(test)]
#[path = "../../tests/npc_world_postgres.rs"]
mod npc_travel_fixture_data;

#[cfg(test)]
mod npc_travel_postgres {
    use super::npc_travel_fixture_data::TEST_SCHEMA;
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

    #[test]
    fn npc_travel_native_postgres_replay_fences_rollback_and_reload() -> TestResult {
        let admin =
            fixture::configured_admin().ok_or("canonical PostgreSQL17.6 must be configured")?;
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
        wrong.connection_generation =
            crate::foundation::ConnectionGeneration::new(2).map_err(debug)?;
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
}
