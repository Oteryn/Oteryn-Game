//! TUTORIAL-1 progress projection; session fences precede every read, including conflicts.
//! The root lock serializes the missing-row case; progress has its own CAS version and
//! does not advance CharacterRevision. Equal-state stale writers still conflict.

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use oteryn_gameplay_quests::tutorial::{TutorialProgress, TutorialStep};
use sqlx::Row;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredTutorialProgress {
    pub version: i64,
    pub progress: TutorialProgress,
    pub pending_islet: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TutorialProgressOutcome {
    Read(Option<StoredTutorialProgress>),
    Written(StoredTutorialProgress),
    Conflict(Option<StoredTutorialProgress>),
    Unchanged(StoredTutorialProgress),
}

impl DurabilityRoot {
    /// Read (`change = None`) or CAS (`expected = 0` means absent). No occurrence identity
    /// is inferred: a caller re-reduces a Conflict against its returned stored progress.
    pub async fn character_tutorial_progress(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        change: Option<(i64, TutorialProgress)>,
    ) -> Result<TutorialProgressOutcome, CharacterProgressionError> {
        if change.is_some_and(|(version, row)| {
            version < 0
                || version == i64::MAX
                || (row.step == TutorialStep::Departed && (row.hunting || row.corruptor))
        }) {
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
                    if let Err(error) = assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        return Ok(Err(error));
                    }
                    let outcome =
                        tutorial_progress_in_transaction(&mut tx, fence.character_id, change)
                            .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(outcome))
                })
            })
            .await?
    }
}

/// Caller has locked the Character root under current session authority in this transaction.
pub(super) async fn tutorial_progress_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: crate::domain::CharacterId,
    change: Option<(i64, TutorialProgress)>,
) -> Result<TutorialProgressOutcome, DurabilityError> {
    let stored = sqlx::query(
                "SELECT version,step,hunting,corruptor,pending_islet FROM game_character_tutorial WHERE character_id=encode($1,'hex')::uuid FOR UPDATE"
            ).bind(character.as_bytes().as_slice()).fetch_optional(&mut **tx).await?
                .map(|row| -> Result<StoredTutorialProgress, DurabilityError> {
                    let step = match row.try_get::<i16,_>("step")? {
                        1 => TutorialStep::Islet,
                        2 => TutorialStep::Island,
                        3 => TutorialStep::Departed,
                        _ => return Err(DurabilityError::InvalidStoredState),
                    };
                    Ok(StoredTutorialProgress { version: row.try_get("version")?, pending_islet:row.try_get("pending_islet")?, progress: TutorialProgress {
                        step, hunting: row.try_get("hunting")?, corruptor: row.try_get("corruptor")?,
                    }})
                }).transpose()?;
    let outcome = match change {
        None => TutorialProgressOutcome::Read(stored),
        Some((expected, _)) if stored.map_or(0, |row| row.version) != expected => {
            TutorialProgressOutcome::Conflict(stored)
        }
        Some((_, progress)) if stored.is_some_and(|row| row.progress == progress) => {
            // Equal state at the CURRENT version is an actual replay; no SQL write.
            TutorialProgressOutcome::Unchanged(stored.ok_or(DurabilityError::InvalidStoredState)?)
        }
        Some((expected, progress)) => {
            let step: i16 = match progress.step {
                TutorialStep::Islet => 1,
                TutorialStep::Island => 2,
                TutorialStep::Departed => 3,
            };
            let version = expected
                .checked_add(1)
                .ok_or(DurabilityError::InvalidStoredState)?;
            let pending_islet = progress.step == TutorialStep::Islet
                && stored.is_none_or(|row| {
                    row.pending_islet || row.progress.step == TutorialStep::Departed
                });
            sqlx::query("INSERT INTO game_character_tutorial(character_id,version,step,hunting,corruptor,pending_islet) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5,$6) ON CONFLICT(character_id) DO UPDATE SET version=EXCLUDED.version,step=EXCLUDED.step,hunting=EXCLUDED.hunting,corruptor=EXCLUDED.corruptor,pending_islet=EXCLUDED.pending_islet")
                        .bind(character.as_bytes().as_slice()).bind(version).bind(step)
                        .bind(progress.hunting).bind(progress.corruptor).bind(pending_islet).execute(&mut **tx).await?;
            TutorialProgressOutcome::Written(StoredTutorialProgress {
                version,
                progress,
                pending_islet,
            })
        }
    };
    Ok(outcome)
}
