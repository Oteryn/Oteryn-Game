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
#[path = "../../tests/support/npc_travel_postgres_fixture.rs"]
pub(crate) mod npc_travel_postgres;
