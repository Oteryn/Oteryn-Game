//! The durable last position of a Character (CHAR-POSITION-1, decision CHAR-POSITION-0,
//! `reviews/OTERYN_GAME_CHAR_POSITION0_LOGOUT_POSITION_DECISION_2026-09-30.md` §3, migration
//! 0080).
//!
//! The row is a Character runtime-state projection (§3.2): it is outside the CharacterRevision
//! chain, with no cause, no receipt and no replay. Every write is one upsert under the
//! composition rule 2 session checks (the reconnect-session row, the runtime-scope assignment and
//! the admission guards), with no `character_root` row lock. It applies only when its order key
//! (Character lease generation, `write_sequence`) is greater than the stored key, and never while
//! a pending respawn exists. Three writers use it:
//!
//! - the periodic write of a live Channel actor ([`DurabilityRoot::write_character_last_position`],
//!   or a batch of them in one transaction, [`DurabilityRoot::write_character_last_positions`]);
//! - the final write inside the terminal release transaction, before `session_state` becomes 3
//!   (`FreshAdmissionStore::with_final_position`);
//! - the placement of a consumed pending respawn, in the transaction that consumes it
//!   (`DurabilityRoot::consume_pending_respawn_placed`).
//!
//! An instance scope (`runtime_scope_kind = 2`) writes nothing: the gameplay fence admits only a
//! Channel scope, and the terminal release skips any other scope.

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_session_fence, numeric_u64,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::CharacterId;
use crate::foundation::{RuntimeScopeRefV1, WorldId};
use sqlx::Row;

type Result<T> = std::result::Result<T, CharacterProgressionError>;

/// A Channel movement cell: `x`, `y` and the legacy `floor` the Channel runtime and the pending
/// respawn record use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LastPositionCell {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}

/// One last position as written (§3.1): the World, the cell, and the frame, map and server
/// artifact digests of the Channel content it was written under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LastPositionRecord {
    pub world_id: WorldId,
    pub cell: LastPositionCell,
    pub coordinate_frame_digest: [u8; 32],
    pub map_revision_digest: [u8; 32],
    pub server_artifact_digest: [u8; 32],
}

/// The §3.2 order key of one write: the writing session's Character lease generation and the
/// write's sequence within that lease.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LastPositionKey {
    pub character_lease_generation: u64,
    pub write_sequence: u64,
}

/// The stored row with its order key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredLastPosition {
    pub record: LastPositionRecord,
    pub key: LastPositionKey,
}

/// What one fenced upsert did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LastPositionWrite {
    /// The row now holds this write.
    Written,
    /// The stored key is equal or greater: a later write is already durable, so this one is
    /// dropped (§3.2 "Order").
    Superseded,
    /// A pending respawn decides the next placement, so nothing is written (§3.2 "Not
    /// written").
    PendingObligation,
}

/// The most periodic writes one batch carries ([`DurabilityRoot::write_character_last_positions`]).
/// A Channel's write pass needs at most `ceil(MAX_CONNECTIONS / LAST_POSITION_WRITE_BATCH)`
/// batches, each one fence read and one write transaction, so its durability work is bounded
/// whatever the number of sessions (`CHARPOS0-RL-01`).
pub const LAST_POSITION_WRITE_BATCH: usize = 32;

/// One periodic write of a batch: the writing session's fence (its
/// `expected_character_revision` is not read), the write's sequence within the session's lease
/// and the record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeriodicLastPosition {
    pub fence: CurrentCharacterGameplayFence,
    pub write_sequence: u64,
    pub record: LastPositionRecord,
}

/// The final write the terminal release commits (§3.2 "At logout"): the record and the write
/// sequence it was captured with. The lease generation is the released session's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinalLastPosition {
    pub record: LastPositionRecord,
    pub write_sequence: u64,
}

impl DurabilityRoot {
    /// §3.2 periodic write of a live Channel actor's position, under the session fence of
    /// `fence` (its `expected_character_revision` is not read: the row takes no root lock).
    pub async fn write_character_last_position(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        write_sequence: u64,
        record: LastPositionRecord,
    ) -> Result<LastPositionWrite> {
        validate_fence(&fence, &record)?;
        if write_sequence == 0 {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if let Err(error) = assert_session_fence(&mut tx, &fence, &node).await? {
                        return Ok(Err(error));
                    }
                    let key = LastPositionKey {
                        character_lease_generation: fence.character_lease_generation,
                        write_sequence,
                    };
                    let outcome =
                        upsert_last_position(&mut tx, fence.character_id.as_bytes(), key, &record)
                            .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(outcome))
                })
            })
            .await?
    }

    /// §3.2 periodic writes of up to [`LAST_POSITION_WRITE_BATCH`] live Channel actors in one
    /// transaction, so a write pass costs one durability pass per batch rather than one per
    /// session (`CHARPOS0-RL-01`). Each write is checked exactly as
    /// [`Self::write_character_last_position`] checks it, under its own session fence, and has
    /// its own outcome, in the order of `writes`: a refused fence or an invalid input refuses
    /// that write only. A storage error, or the pass deadline, fails the whole batch and commits
    /// none of it, so every write of it stays due.
    pub async fn write_character_last_positions(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        writes: Vec<PeriodicLastPosition>,
    ) -> Result<Vec<Result<LastPositionWrite>>> {
        if writes.len() > LAST_POSITION_WRITE_BATCH {
            return Err(CharacterProgressionError::InvalidInput);
        }
        if writes.is_empty() {
            return Ok(Vec::new());
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        Ok(self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let mut outcomes = Vec::with_capacity(writes.len());
                    for write in writes {
                        outcomes.push(write_periodic(&mut tx, &node, write).await?);
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(outcomes)
                })
            })
            .await?)
    }

    /// §3.3: the Character's last position, read under the session fence of `fence` when a new
    /// runtime actor is admitted. `None` when no row exists (a first login).
    pub async fn read_character_last_position(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<Option<StoredLastPosition>> {
        if fence.character_lease_generation == 0
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if let Err(error) = assert_session_fence(&mut tx, &fence, &node).await? {
                        return Ok(Err(error));
                    }
                    let stored = load_last_position(&mut tx, fence.character_id).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(stored))
                })
            })
            .await?
    }
}

/// One write of a batch, inside its transaction (which holds the recovery fence and the
/// admission relation locks): the checks and upsert of
/// [`DurabilityRoot::write_character_last_position`]. `Ok(Err(_))` refuses this write only.
async fn write_periodic(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    write: PeriodicLastPosition,
) -> std::result::Result<Result<LastPositionWrite>, DurabilityError> {
    if let Err(error) = validate_fence(&write.fence, &write.record) {
        return Ok(Err(error));
    }
    if write.write_sequence == 0 {
        return Ok(Err(CharacterProgressionError::InvalidInput));
    }
    if let Err(error) = assert_session_fence(tx, &write.fence, node).await? {
        return Ok(Err(error));
    }
    let key = LastPositionKey {
        character_lease_generation: write.fence.character_lease_generation,
        write_sequence: write.write_sequence,
    };
    let outcome =
        upsert_last_position(tx, write.fence.character_id.as_bytes(), key, &write.record).await?;
    Ok(Ok(outcome))
}

fn validate_fence(
    fence: &CurrentCharacterGameplayFence,
    record: &LastPositionRecord,
) -> Result<()> {
    match fence.runtime_scope {
        RuntimeScopeRefV1::Channel { world_id, .. }
            if fence.character_lease_generation != 0 && world_id == record.world_id =>
        {
            Ok(())
        }
        _ => Err(CharacterProgressionError::InvalidInput),
    }
}

/// The one §3.2 upsert, inside a transaction that already holds the session fence (or is the
/// terminal release). It applies only when `key` is greater than the stored key and no pending
/// respawn exists.
pub(super) async fn upsert_last_position(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character_id: &[u8; 16],
    key: LastPositionKey,
    record: &LastPositionRecord,
) -> std::result::Result<LastPositionWrite, DurabilityError> {
    if key.character_lease_generation == 0 || key.write_sequence == 0 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let pending: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM game_character_pending_respawns \
          WHERE character_id = encode($1,'hex')::uuid)",
    )
    .bind(character_id.as_slice())
    .fetch_one(&mut **tx)
    .await?;
    if pending {
        return Ok(LastPositionWrite::PendingObligation);
    }
    let written = sqlx::query(
        "INSERT INTO game_character_last_positions AS p \
            (character_id, world_id, tile_x, tile_y, tile_floor, coordinate_frame_digest, \
             map_revision_digest, server_artifact_digest, character_lease_generation, \
             write_sequence) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3, $4, $5, $6, $7, $8, \
                 $9::text::numeric(20,0), $10::text::numeric(20,0)) \
         ON CONFLICT (character_id) DO UPDATE SET \
             world_id = EXCLUDED.world_id, tile_x = EXCLUDED.tile_x, \
             tile_y = EXCLUDED.tile_y, tile_floor = EXCLUDED.tile_floor, \
             coordinate_frame_digest = EXCLUDED.coordinate_frame_digest, \
             map_revision_digest = EXCLUDED.map_revision_digest, \
             server_artifact_digest = EXCLUDED.server_artifact_digest, \
             character_lease_generation = EXCLUDED.character_lease_generation, \
             write_sequence = EXCLUDED.write_sequence \
         WHERE (p.character_lease_generation, p.write_sequence) \
             < (EXCLUDED.character_lease_generation, EXCLUDED.write_sequence)",
    )
    .bind(character_id.as_slice())
    .bind(record.world_id.as_bytes().as_slice())
    .bind(record.cell.x)
    .bind(record.cell.y)
    .bind(record.cell.floor)
    .bind(record.coordinate_frame_digest.as_slice())
    .bind(record.map_revision_digest.as_slice())
    .bind(record.server_artifact_digest.as_slice())
    .bind(key.character_lease_generation.to_string())
    .bind(key.write_sequence.to_string())
    .execute(&mut **tx)
    .await?;
    Ok(if written.rows_affected() == 1 {
        LastPositionWrite::Written
    } else {
        LastPositionWrite::Superseded
    })
}

async fn load_last_position(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character_id: CharacterId,
) -> std::result::Result<Option<StoredLastPosition>, DurabilityError> {
    let row = sqlx::query(
        "SELECT world_id::text, tile_x, tile_y, tile_floor, coordinate_frame_digest, \
                map_revision_digest, server_artifact_digest, \
                character_lease_generation::text, write_sequence::text \
           FROM game_character_last_positions \
          WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(character_id.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    row.as_ref().map(decode_row).transpose()
}

fn decode_row(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<StoredLastPosition, DurabilityError> {
    let digest = |column: &str| -> std::result::Result<[u8; 32], DurabilityError> {
        row.try_get::<Vec<u8>, _>(column)?
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)
    };
    let world: String = row.try_get("world_id")?;
    let world_id = WorldId::decode(&super::character_progression::uuid_text(&world)?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(StoredLastPosition {
        record: LastPositionRecord {
            world_id,
            cell: LastPositionCell {
                x: row.try_get("tile_x")?,
                y: row.try_get("tile_y")?,
                floor: row.try_get("tile_floor")?,
            },
            coordinate_frame_digest: digest("coordinate_frame_digest")?,
            map_revision_digest: digest("map_revision_digest")?,
            server_artifact_digest: digest("server_artifact_digest")?,
        },
        key: LastPositionKey {
            character_lease_generation: numeric_u64(row, "character_lease_generation")?,
            write_sequence: numeric_u64(row, "write_sequence")?,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::LastPositionKey;

    /// §3.2 "Order": a later lease always orders after an earlier one, whatever its sequence.
    #[test]
    fn key_orders_by_lease_then_sequence() {
        let key = |character_lease_generation, write_sequence| LastPositionKey {
            character_lease_generation,
            write_sequence,
        };
        assert!(key(2, 1) > key(1, u64::MAX));
        assert!(key(1, 2) > key(1, 1));
        assert!(key(1, 1) == key(1, 1));
    }
}
