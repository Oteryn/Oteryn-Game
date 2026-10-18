//! TUTORIAL-1: the durable Newhaven tutorial progress of a Character (CP D1426, migration 0130).
//!
//! The row is a Character runtime-state projection, outside the CharacterRevision chain, with no
//! cause and no receipt. Every read and write runs under the composition rule 2 session checks
//! (the reconnect-session row, the runtime-scope assignment and the admission guards) of the
//! caller's [`CurrentCharacterGameplayFence`], as the last position does, so a stale connection,
//! lease, scope or session reads and writes nothing.
//!
//! A write is a compare-and-set on `progress_version`: the caller names the version it read (`0`
//! for no row) and the row is locked before it is compared. Of two concurrent writers from the
//! same version exactly one writes; the other gets [`TutorialProgressWrite::Conflict`] with the
//! stored progress. A replay of a committed write (the same progress at the next version) is
//! [`TutorialProgressWrite::Replayed`] and writes nothing. A reconnect or a node restart reads the
//! stored step back, so the tutorial resumes where it was.

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
use crate::domain::tutorial::{TutorialProgress, TutorialStep};
use crate::foundation::RuntimeScopeRefV1;
use sqlx::Row;

type Result<T> = std::result::Result<T, CharacterProgressionError>;

/// A stored progress and its compare-and-set version (at least 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredTutorialProgress {
    pub progress: TutorialProgress,
    pub version: u64,
}

/// The outcome of one tutorial progress write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TutorialProgressWrite {
    /// The progress is stored at `version`.
    Written { version: u64 },
    /// The same progress was already stored at `version` (a replay of a committed write).
    Replayed { version: u64 },
    /// The stored version is not the expected one: another writer won. `None` is no row.
    Conflict {
        stored: Option<StoredTutorialProgress>,
    },
}

impl DurabilityRoot {
    /// Reads the tutorial progress of the fence's Character; `None` if it never entered the
    /// tutorial.
    pub async fn read_character_tutorial_progress(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<Option<StoredTutorialProgress>> {
        validate_fence(&fence)?;
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
                    if let Err(error) =
                        assert_session_fence(&mut tx, &fence.item_fence(), &node).await?
                    {
                        return Ok(Err(error));
                    }
                    let stored = load_progress(&mut tx, fence.character_id, false).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(stored))
                })
            })
            .await?
    }

    /// Stores `progress` if the stored version is `expected_version` (`0`: no row).
    pub async fn write_character_tutorial_progress(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        expected_version: u64,
        progress: TutorialProgress,
    ) -> Result<TutorialProgressWrite> {
        validate_fence(&fence)?;
        if !progress.is_valid() || expected_version == u64::MAX {
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
                    if let Err(error) =
                        assert_session_fence(&mut tx, &fence.item_fence(), &node).await?
                    {
                        return Ok(Err(error));
                    }
                    let outcome = compare_and_set(
                        &mut tx,
                        fence.character_id,
                        fence.character_lease_generation,
                        expected_version,
                        progress,
                    )
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(outcome))
                })
            })
            .await?
    }
}

fn validate_fence(fence: &CurrentCharacterGameplayFence) -> Result<()> {
    if fence.character_lease_generation == 0
        || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
    {
        return Err(CharacterProgressionError::InvalidInput);
    }
    Ok(())
}

async fn compare_and_set(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character_id: CharacterId,
    lease: u64,
    expected_version: u64,
    progress: TutorialProgress,
) -> std::result::Result<TutorialProgressWrite, DurabilityError> {
    let next = expected_version + 1;
    let stored = load_progress(tx, character_id, true).await?;
    let current = stored.map_or(0, |stored| stored.version);
    if current != expected_version {
        return Ok(match stored {
            Some(stored) if stored.version == next && stored.progress == progress => {
                TutorialProgressWrite::Replayed { version: next }
            }
            _ => TutorialProgressWrite::Conflict { stored },
        });
    }
    let written = if stored.is_none() {
        sqlx::query(
            "INSERT INTO game_character_tutorial_progress \
                (character_id, tutorial_step, hunting_hint_pending, corruptor_loot_pending, \
                 progress_version, character_lease_generation) \
             VALUES (encode($1,'hex')::uuid, $2, $3, $4, 1, $5::text::numeric(20,0)) \
             ON CONFLICT (character_id) DO NOTHING",
        )
        .bind(character_id.as_bytes().as_slice())
        .bind(progress.step.code())
        .bind(progress.hunting_hint_pending)
        .bind(progress.corruptor_loot_pending)
        .bind(lease.to_string())
        .execute(&mut **tx)
        .await?
    } else {
        sqlx::query(
            "UPDATE game_character_tutorial_progress SET tutorial_step = $2, \
                    hunting_hint_pending = $3, corruptor_loot_pending = $4, \
                    progress_version = progress_version + 1, \
                    character_lease_generation = $5::text::numeric(20,0) \
              WHERE character_id = encode($1,'hex')::uuid \
                AND progress_version = $6::text::numeric(20,0)",
        )
        .bind(character_id.as_bytes().as_slice())
        .bind(progress.step.code())
        .bind(progress.hunting_hint_pending)
        .bind(progress.corruptor_loot_pending)
        .bind(lease.to_string())
        .bind(expected_version.to_string())
        .execute(&mut **tx)
        .await?
    };
    if written.rows_affected() == 1 {
        return Ok(TutorialProgressWrite::Written { version: next });
    }
    // A concurrent first insert won between the locking read and the insert.
    let stored = load_progress(tx, character_id, true).await?;
    Ok(match stored {
        Some(stored) if stored.version == next && stored.progress == progress => {
            TutorialProgressWrite::Replayed { version: next }
        }
        _ => TutorialProgressWrite::Conflict { stored },
    })
}

async fn load_progress(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character_id: CharacterId,
    for_update: bool,
) -> std::result::Result<Option<StoredTutorialProgress>, DurabilityError> {
    let sql = if for_update {
        "SELECT tutorial_step, hunting_hint_pending, corruptor_loot_pending, \
                progress_version::text \
           FROM game_character_tutorial_progress \
          WHERE character_id = encode($1,'hex')::uuid FOR UPDATE"
    } else {
        "SELECT tutorial_step, hunting_hint_pending, corruptor_loot_pending, \
                progress_version::text \
           FROM game_character_tutorial_progress \
          WHERE character_id = encode($1,'hex')::uuid"
    };
    let row = sqlx::query(sql)
        .bind(character_id.as_bytes().as_slice())
        .fetch_optional(&mut **tx)
        .await?;
    row.as_ref().map(decode_row).transpose()
}

fn decode_row(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<StoredTutorialProgress, DurabilityError> {
    let step = TutorialStep::from_code(row.try_get("tutorial_step")?)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let progress = TutorialProgress {
        step,
        hunting_hint_pending: row.try_get("hunting_hint_pending")?,
        corruptor_loot_pending: row.try_get("corruptor_loot_pending")?,
    };
    let version = numeric_u64(row, "progress_version")?;
    if !progress.is_valid() || version == 0 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(StoredTutorialProgress { progress, version })
}
