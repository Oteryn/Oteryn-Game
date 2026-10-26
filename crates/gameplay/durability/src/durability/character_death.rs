//! Typed, fenced Character death commits (DEATH-1,
//! `reviews/OTERYN_GAME_DEATH0_CHARACTER_DEATH_RECEIPT_DECISION_2026-09-28.md`
//! §3.5 and the Reference first player death decision §4.3).
//!
//! One transaction keyed by the `PlayerDeathOccurrence` advances the global
//! CharacterRevision with exactly one death receipt: the D58 experience loss,
//! the consumption of every held blessing and the pending respawn at the
//! recorded position, and empties the monk Harmony and remaining forced Serene
//! time (SPELL-D8 §8.2 cross-owner item, migration 0026).  It writes Character
//! state only.  Until DEATH-3 a death selects no Amulet of Loss and loses no
//! item. PREM-2b supplies durable promotion and per-use Premium under the fence
//! for the D66 reduction; PREM-5 supplies the held blessing set.
//!
//! DEATH-2 adds the consumption of the pending respawn (DEATH-0 §3.4) and the
//! sequenced settlement the Channel owner runs off its lane for one runtime
//! death: commit (or replay) the death in the Character's revision slot.  The
//! death's own pending respawn stays recorded until the caller has placed the
//! player at its respawn position and consumes it
//! ([`DurabilityRoot::consume_pending_respawn`]), so a restart between the
//! commit and the placement never loses the recorded respawn (Reference first
//! player death §4.5, DEATH-RESTART-SAFETY-1).

use super::character_authority::{
    CharacterAuthorityError, ReconciledCharacterAuthority, assert_recovery_fence,
};
use super::character_build::{
    BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildFormula, CommittedBuildChange,
    DurableBuildState, SKILLS,
};
use super::character_last_position::{FinalLastPosition, LastPositionKey, upsert_last_position};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, ProgressionInitializationRequest,
    assert_gameplay_fence, numeric_u64, policy_digest, state_matches_root, stored_context_matches,
    uuid_text, valid_revision,
};
use super::character_revision_sequencer::{CharacterRevisionSequencer, RevisionSlot};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::death::calculate_death_build_family_loss;
use crate::domain::progression::{
    CurrentProgressionSnapshot, FiniteProgressionPolicy, ProgressionOperation,
    ProgressionRevisionContext, calculate_progression,
};
use crate::domain::{CharacterId, CharacterRevision};
use crate::foundation::{ChannelId, RuntimeScopeRefV1, WorldId};
use oteryn_simulation_determinism::ExactI64;
use sha2::{Digest, Sha256};
use sqlx::Row;

type Result<T> = std::result::Result<T, CharacterProgressionError>;
const DEATH_BINDING_VERSION: u8 = 1;
const DEATH_BUILD_BINDING_VERSION: u8 = 2;
const DEATH_BINDING_DOMAIN: &[u8] = b"oteryn:character-death";
/// 0016 storage bounds.
const MAX_POSITION_BYTES: usize = 128;
const MAX_MAP_REVISION_BYTES: usize = 512;
const MAX_BLESSINGS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlayerDeathOccurrence([u8; 16]);

impl PlayerDeathOccurrence {
    pub fn from_bytes(bytes: [u8; 16]) -> Result<Self> {
        if bytes[6] >> 4 != 7 || bytes[8] & 0xc0 != 0x80 {
            return Err(CharacterProgressionError::InvalidInput);
        }
        Ok(Self(bytes))
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

/// The durable destination a resumed DEATH-3 item workflow drops at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeathCell {
    pub world_id: WorldId,
    pub channel_id: ChannelId,
    pub spatial_position: Vec<u8>,
    pub map_revision: String,
}

/// Complete semantic input of one death.  `held_blessings` is the intent's
/// view of the held set in strictly ascending byte order; it must equal the
/// durable set when a new death commits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterDeathRequest<const N: usize> {
    pub occurrence: PlayerDeathOccurrence,
    pub context: ProgressionRevisionContext<String>,
    pub policy_revision: String,
    pub reward_revision: String,
    pub policy: FiniteProgressionPolicy<String, N>,
    pub held_blessings: Vec<String>,
    /// Qualified Open-PvP consequences; empty until the runtime classifier is enabled.
    pub pvp_contributors: Vec<super::pvp_death_consequences::PvpDeathContributor>,
    pub death_cell: DeathCell,
    pub respawn_position: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedCharacterDeath {
    pub occurrence: PlayerDeathOccurrence,
    pub character_id: CharacterId,
    pub original_character_revision: CharacterRevision,
    pub committed_character_revision: CharacterRevision,
    pub level_before: u32,
    pub level_after: u32,
    pub experience_before: ExactI64,
    pub experience_after: ExactI64,
    pub experience_lost: ExactI64,
    pub blessings_before: Vec<String>,
    pub blessings_after: Vec<String>,
    pub death_cell: DeathCell,
    pub respawn_position: Vec<u8>,
    /// None means no build changed, including a retained pre-build-loss death.
    pub build_loss: Option<CommittedDeathBuildLoss>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedDeathBuildLoss {
    pub before: DurableBuildState,
    pub after: DurableBuildState,
    pub formula_digest: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharacterDeathOutcome {
    Committed(CommittedCharacterDeath),
    AlreadyCommitted(CommittedCharacterDeath),
}

/// An owner-prepared checkpoint, not proof of current authority or that all live
/// training has been drained. The caller retains it until settlement and supplies
/// its original formula digest. The normal build/death writers independently
/// assert current DB fences; receipt history never replaces those checks.
pub struct DeathTrainingFlush<'a> {
    pub fence: CurrentCharacterGameplayFence,
    pub request: &'a BuildChangeRequest,
    pub formula: &'a dyn BuildFormula,
    pub formula_digest: [u8; 32],
}

/// A pending respawn (DEATH-0 §3.4): the one a read found, or the one a consumption deleted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumedRespawn {
    pub occurrence: PlayerDeathOccurrence,
    pub respawn_position: Vec<u8>,
}

mod adventurer;
mod commit;
pub mod promotion;
mod settlement;

impl DurabilityRoot {
    /// Commit one player death.  Exact occurrence replay returns the first
    /// receipt without reacquiring session authority; changed semantic reuse
    /// conflicts before replay.  A new occurrence is fenced exactly like
    /// `commit_character_experience` and serializes with it on the
    /// `character_root` row lock.  The receipt is inserted before the
    /// blessings it consumes are deleted (0016 admission trigger).
    /// Runtime callers reach it only through a
    /// [`RevisionSlot`](super::character_revision_sequencer::RevisionSlot) (CHAR-REV-SEQ-1).
    pub async fn commit_character_death<const N: usize>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: CharacterDeathRequest<N>,
    ) -> Result<CharacterDeathOutcome> {
        self.commit_character_death_inner(
            authority,
            node,
            fence,
            request,
            None,
            promotion::DeathPremiumState::default(),
        )
        .await
    }

    /// Build-aware death writer. The formula comes from the owning content loader;
    /// its content revision is checked against independently locked current state.
    /// The sequenced runtime caller is integrated in its allocated wiring slot.
    pub async fn commit_character_death_with_build<const N: usize>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: CharacterDeathRequest<N>,
        formula: &dyn BuildFormula,
    ) -> Result<CharacterDeathOutcome> {
        self.commit_character_death_inner(
            authority,
            node,
            fence,
            request,
            Some(formula),
            promotion::DeathPremiumState::default(),
        )
        .await
    }

    /// Consume the Character's pending respawn (DEATH-0 §3.4) under the
    /// fences of a death: the recovery fence, the admission relation locks
    /// and the full gameplay fence with its `character_root` row lock, at the
    /// fence's expected revision.  `occurrence` names the death being
    /// respawned; `None` consumes whichever respawn is pending (the next
    /// admission after a restart).  Returns the deleted row, or `None` when no
    /// respawn (of that occurrence) is pending, so a retry deletes nothing
    /// twice.  It advances no CharacterRevision; a stale fence deletes nothing.
    pub async fn consume_pending_respawn(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        occurrence: Option<PlayerDeathOccurrence>,
    ) -> Result<Option<ConsumedRespawn>> {
        self.pending_respawn_under_fence(authority, node, fence, occurrence, true, None)
            .await
    }

    /// CHAR-POSITION-0 §3.3 step 1: [`Self::consume_pending_respawn`] for an admission that
    /// placed the actor at `placed`. The same transaction upserts the Character's last position
    /// with the placed cell under the admitted session's lease key and `write_sequence`, so a
    /// crash right after placement never puts the Character back at its death cell. The upsert
    /// runs only when the respawn was deleted.
    pub async fn consume_pending_respawn_placed(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        occurrence: Option<PlayerDeathOccurrence>,
        placed: FinalLastPosition,
    ) -> Result<Option<ConsumedRespawn>> {
        validate_placed(&fence, &placed)?;
        self.pending_respawn_under_fence(authority, node, fence, occurrence, true, Some(placed))
            .await
    }

    /// DEATH-RESTART-SAFETY-1 (#2004 Codex P1 4248558737): place a living
    /// session's dead actor for the respawn of `occurrence` while the durable
    /// fence of [`Self::consume_pending_respawn`] is held, and consume the row
    /// only after that placement, in the same transaction.  `place` runs
    /// inside the transaction after the recovery fence, the admission relation
    /// locks and the full gameplay fence were asserted: the session row, the
    /// scope assignment and the node registration stay share-locked and the
    /// Character root row-locked until the transaction ends, so no termination,
    /// supersession, scope move or node revoke commits before `place` returns.
    /// It receives the pending row of `occurrence` (`None` when an earlier
    /// attempt already consumed it) and must do no I/O.  `true` (it placed the
    /// actor) deletes that row; `false` deletes nothing.  A refused fence never
    /// calls `place`.
    pub async fn place_and_consume_pending_respawn<P, F>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        occurrence: PlayerDeathOccurrence,
        place: P,
    ) -> Result<bool>
    where
        P: FnOnce(Option<ConsumedRespawn>) -> F + Send,
        F: std::future::Future<Output = bool> + Send,
    {
        self.place_and_consume_under_fence(authority, node, fence, occurrence, None, place)
            .await
    }

    /// CHAR-POSITION-0 §3.2-§3.3 step 1 with DEATH-RESTART-SAFETY-1:
    /// [`Self::place_and_consume_pending_respawn`] for an in-session respawn whose `place`
    /// puts the actor at `placed`. When `place` placed the actor and the row of `occurrence`
    /// was deleted, the same transaction, under the same fence, upserts `placed` as the
    /// Character's last position under the fence's lease generation and
    /// `placed.write_sequence`, so a crash right after the placement never puts the Character
    /// back at its death cell. A placement that deletes nothing (an earlier attempt consumed
    /// the row, and its transaction wrote that attempt's placed cell) or a `place` that returns
    /// `false` writes no position. `placed` of another World, or with a zero sequence, is
    /// refused before `place` is called.
    pub async fn place_and_consume_pending_respawn_placed<P, F>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        occurrence: PlayerDeathOccurrence,
        placed: FinalLastPosition,
        place: P,
    ) -> Result<bool>
    where
        P: FnOnce(Option<ConsumedRespawn>) -> F + Send,
        F: std::future::Future<Output = bool> + Send,
    {
        validate_placed(&fence, &placed)?;
        self.place_and_consume_under_fence(authority, node, fence, occurrence, Some(placed), place)
            .await
    }

    async fn place_and_consume_under_fence<P, F>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        occurrence: PlayerDeathOccurrence,
        placed: Option<FinalLastPosition>,
        place: P,
    ) -> Result<bool>
    where
        P: FnOnce(Option<ConsumedRespawn>) -> F + Send,
        F: std::future::Future<Output = bool> + Send,
    {
        if fence.character_lease_generation == 0
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let mut context = (Some(place), std::marker::PhantomData::<F>);

        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if let Err(error) = assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        return Ok(Err(error));
                    }
                    let row = sqlx::query(
                        "SELECT death_occurrence_id::text, respawn_position \
                           FROM game_character_pending_respawns \
                          WHERE character_id = encode($1,'hex')::uuid \
                            AND death_occurrence_id = encode($2,'hex')::uuid",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(occurrence.0.as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let pending = match row {
                        None => None,
                        Some(row) => Some(ConsumedRespawn {
                            occurrence,
                            respawn_position: row.try_get("respawn_position")?,
                        }),
                    };
                    let Some(place) = context.0.take() else {
                        return Ok(Err(CharacterProgressionError::InvalidInput));
                    };
                    if !place(pending).await {
                        return Ok(Ok(false));
                    }
                    let deleted = sqlx::query(
                        "DELETE FROM game_character_pending_respawns \
                          WHERE character_id = encode($1,'hex')::uuid \
                            AND death_occurrence_id = encode($2,'hex')::uuid",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(occurrence.0.as_slice())
                    .execute(&mut *tx)
                    .await?;
                    // CHAR-POSITION-0 §3.3 step 1: the placed cell, after the row is gone (the
                    // upsert writes nothing while a pending respawn exists).
                    if let Some(placed) = placed
                        && deleted.rows_affected() == 1
                    {
                        let key = LastPositionKey {
                            character_lease_generation: fence.character_lease_generation,
                            write_sequence: placed.write_sequence,
                        };
                        upsert_last_position(
                            &mut tx,
                            fence.character_id.as_bytes(),
                            key,
                            &placed.record,
                        )
                        .await?;
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(true))
                })
            })
            .await?
    }

    /// Read the Character's pending respawn (DEATH-0 §3.4) under the same
    /// fences as [`Self::consume_pending_respawn`], deleting nothing, so an
    /// admission places the actor at its recorded position before the
    /// obligation is consumed.
    pub async fn pending_respawn(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<Option<ConsumedRespawn>> {
        self.pending_respawn_under_fence(authority, node, fence, None, false, None)
            .await
    }

    async fn pending_respawn_under_fence(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        occurrence: Option<PlayerDeathOccurrence>,
        consume: bool,
        placed: Option<FinalLastPosition>,
    ) -> Result<Option<ConsumedRespawn>> {
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
                    if let Err(error) = assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        return Ok(Err(error));
                    }
                    let row = sqlx::query(if consume {
                        "DELETE FROM game_character_pending_respawns \
                          WHERE character_id = encode($1,'hex')::uuid \
                            AND ($2::bytea IS NULL \
                                 OR death_occurrence_id = encode($2,'hex')::uuid) \
                         RETURNING death_occurrence_id::text, respawn_position"
                    } else {
                        "SELECT death_occurrence_id::text, respawn_position \
                           FROM game_character_pending_respawns \
                          WHERE character_id = encode($1,'hex')::uuid \
                            AND ($2::bytea IS NULL \
                                 OR death_occurrence_id = encode($2,'hex')::uuid)"
                    })
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(occurrence.map(|occurrence| occurrence.0.to_vec()))
                    .fetch_optional(&mut *tx)
                    .await?;
                    let consumed = match row {
                        None => None,
                        Some(row) => {
                            let id: String = row.try_get("death_occurrence_id")?;
                            Some(ConsumedRespawn {
                                occurrence: PlayerDeathOccurrence::from_bytes(uuid_text(&id)?)
                                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                                respawn_position: row.try_get("respawn_position")?,
                            })
                        }
                    };
                    if let (Some(placed), Some(_)) = (placed, &consumed) {
                        let key = LastPositionKey {
                            character_lease_generation: fence.character_lease_generation,
                            write_sequence: placed.write_sequence,
                        };
                        upsert_last_position(
                            &mut tx,
                            fence.character_id.as_bytes(),
                            key,
                            &placed.record,
                        )
                        .await?;
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(consumed))
                })
            })
            .await?
    }

    /// Read a retained death outcome after a lost response, before respawn.
    /// This proves only what committed for the occurrence and never
    /// reacquires gameplay authority.
    pub async fn reconcile_character_death(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        occurrence: PlayerDeathOccurrence,
    ) -> Result<Option<CommittedCharacterDeath>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let record = load_receipt(&mut tx, occurrence)
                        .await?
                        .as_ref()
                        .map(decode_receipt)
                        .transpose()?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(record))
                })
            })
            .await?
    }
}

fn validate_death_training_input(
    death_fence: CurrentCharacterGameplayFence,
    training: &DeathTrainingFlush<'_>,
) -> Result<()> {
    if (CurrentCharacterGameplayFence {
        expected_character_revision: death_fence.expected_character_revision,
        ..training.fence
    }) != death_fence
    {
        return Err(CharacterProgressionError::AuthorityRejected);
    }
    if training.request.cause != BuildCause::Training
        || training.request.pruned_stance.is_some()
        || training.formula_digest != training.formula.digest()
    {
        return Err(CharacterProgressionError::InvalidInput);
    }
    Ok(())
}

/// Checks historical binding only, never current session authority. The real
/// writer must still compare its complete command binding (including formula).
fn validate_death_training_receipt(
    character: CharacterId,
    expected: CharacterRevision,
    request: &BuildChangeRequest,
    receipt: &CommittedBuildChange,
) -> Result<()> {
    if receipt.character_id != character
        || receipt.original_character_revision != expected
        || receipt.occurrence != request.occurrence
        || receipt.cause != BuildCause::Training
        || receipt.before != request.before
        || receipt.after != request.after
        || receipt.pruned_stance != request.pruned_stance
        || receipt.original_character_revision.get().checked_add(1)
            != Some(receipt.committed_character_revision.get())
    {
        return Err(CharacterProgressionError::ConflictingOccurrence);
    }
    Ok(())
}

/// Actual ordering used above. Callbacks are the existing sequenced writer and
/// receipt reader; deterministic tests cover control flow only, not DB fencing.
async fn settle_death_training_flush<W, R>(
    character: CharacterId,
    expected: CharacterRevision,
    request: &BuildChangeRequest,
    mut write: W,
    mut reconcile: R,
) -> Result<CommittedBuildChange>
where
    W: AsyncFnMut(Option<CharacterRevision>) -> Result<BuildCommitOutcome>,
    R: AsyncFnMut() -> Result<Option<CommittedBuildChange>>,
{
    let retained = reconcile().await?;
    if let Some(receipt) = &retained {
        validate_death_training_receipt(character, expected, request, receipt)?;
    }
    // Preserve the prepared binding even for a new checkpoint: the slot must
    // not silently rebase stale training onto its current cursor.
    let outcome = match write(Some(expected)).await {
        Ok(outcome) => outcome,
        Err(error @ CharacterProgressionError::Unavailable(_)) => {
            let Some(receipt) = reconcile().await? else {
                return Err(error);
            };
            validate_death_training_receipt(character, expected, request, &receipt)?;
            // Reading history alone cannot approve the flush: exact replay through
            // the writer compares the original revision/formula command binding
            // and advances the slot's cursor before the death is allowed.
            write(Some(receipt.original_character_revision)).await?
        }
        Err(error) => return Err(error),
    };
    let (BuildCommitOutcome::Committed(receipt) | BuildCommitOutcome::AlreadyCommitted(receipt)) =
        outcome;
    validate_death_training_receipt(character, expected, request, &receipt)?;
    Ok(receipt)
}

async fn slot_cursor(
    slot: &mut RevisionSlot,
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> Result<CharacterRevision> {
    slot.cursor(root, authority)
        .await
        .map_err(|error| match error {
            CharacterAuthorityError::Unavailable(error) => {
                CharacterProgressionError::Unavailable(error)
            }
            _ => CharacterProgressionError::AuthorityRejected,
        })
}

/// CHAR-POSITION-0 §3.2: a placed cell is written only in the fence's own Channel World and
/// under a nonzero sequence.
fn validate_placed(
    fence: &CurrentCharacterGameplayFence,
    placed: &FinalLastPosition,
) -> Result<()> {
    match fence.runtime_scope {
        RuntimeScopeRefV1::Channel { world_id, .. }
            if world_id == placed.record.world_id && placed.write_sequence != 0 =>
        {
            Ok(())
        }
        _ => Err(CharacterProgressionError::InvalidInput),
    }
}

fn validate_request<const N: usize>(
    fence: &CurrentCharacterGameplayFence,
    request: &CharacterDeathRequest<N>,
) -> Result<()> {
    let cell = &request.death_cell;
    let blessings_canonical = request.held_blessings.len() <= MAX_BLESSINGS
        && request.held_blessings.iter().all(|key| valid_revision(key))
        && request
            .held_blessings
            .windows(2)
            .all(|pair| pair[0].as_bytes() < pair[1].as_bytes());
    if fence.character_lease_generation == 0
        || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        || !all_revisions(request).all(valid_revision)
        || request.context != request.policy.context
        || request.policy_revision != request.policy.policy_revision
        || request.reward_revision != request.policy.reward_revision
        || !blessings_canonical
        || !(1..=MAX_POSITION_BYTES).contains(&cell.spatial_position.len())
        || !(1..=MAX_MAP_REVISION_BYTES).contains(&cell.map_revision.len())
        || !(1..=MAX_POSITION_BYTES).contains(&request.respawn_position.len())
    {
        return Err(CharacterProgressionError::InvalidInput);
    }
    Ok(())
}

fn all_revisions<const N: usize>(request: &CharacterDeathRequest<N>) -> impl Iterator<Item = &str> {
    [
        request.context.profile.as_str(),
        request.context.ruleset.as_str(),
        request.context.content.as_str(),
        request.context.simulation.as_str(),
        request.context.evidence.as_str(),
        request.context.declaration.as_str(),
        request.policy_revision.as_str(),
        request.reward_revision.as_str(),
        request.policy.death_policy_revision.as_str(),
        request.policy.declared_difference_revision.as_str(),
    ]
    .into_iter()
}

fn push_bytes(semantic: &mut Vec<u8>, bytes: &[u8]) -> Result<()> {
    let length = u16::try_from(bytes.len()).map_err(|_| CharacterProgressionError::InvalidInput)?;
    semantic.extend_from_slice(&length.to_be_bytes());
    semantic.extend_from_slice(bytes);
    Ok(())
}

/// Digest of the complete death intent (0016 §3.1): character, original
/// revision, occurrence, policy contents and revisions, blessings held, the
/// promotion and Premium evaluation, the item-effect inputs and the death
/// cell and respawn position.  Retry-local session authority is excluded.
fn command_binding<const N: usize>(
    fence: &CurrentCharacterGameplayFence,
    request: &CharacterDeathRequest<N>,
) -> Result<Vec<u8>> {
    let mut semantic = Vec::new();
    semantic.extend_from_slice(DEATH_BINDING_DOMAIN);
    semantic.push(DEATH_BINDING_VERSION);
    semantic.extend_from_slice(&request.occurrence.0);
    semantic.extend_from_slice(fence.character_id.as_bytes());
    semantic.extend_from_slice(&fence.expected_character_revision.get().to_be_bytes());
    semantic.extend_from_slice(&policy_digest(&request.policy)?);
    for revision in all_revisions(request) {
        push_bytes(&mut semantic, revision.as_bytes())?;
    }
    semantic.push(
        u8::try_from(request.held_blessings.len())
            .map_err(|_| CharacterProgressionError::InvalidInput)?,
    );
    for blessing in &request.held_blessings {
        push_bytes(&mut semantic, blessing.as_bytes())?;
    }
    // Promotion with current Premium: not evaluable before Premium activation.
    semantic.push(0);
    // Amulet of Loss state, equipment/backpack snapshot and RNG stream: none
    // until DEATH-3 (death decision §4.4 delivery gap).
    semantic.push(0);
    semantic.extend_from_slice(request.death_cell.world_id.as_bytes());
    semantic.extend_from_slice(request.death_cell.channel_id.as_bytes());
    push_bytes(&mut semantic, &request.death_cell.spatial_position)?;
    push_bytes(&mut semantic, request.death_cell.map_revision.as_bytes())?;
    push_bytes(&mut semantic, &request.respawn_position)?;
    super::pvp_death_request::bind_contributors(
        &mut semantic,
        fence.character_id,
        &request.pvp_contributors,
    )?;
    let digest: [u8; 32] = Sha256::digest(&semantic).into();
    let mut binding = Vec::with_capacity(33);
    binding.push(DEATH_BINDING_VERSION);
    binding.extend_from_slice(&digest);
    Ok(binding)
}

fn build_families(state: &DurableBuildState) -> [(u16, u64); 8] {
    let mut families = [(0, 0); 8];
    families[0] = state.magic();
    families[1..].copy_from_slice(&state.skills());
    families
}

fn prepare_death_build_loss(
    formula: Option<&dyn BuildFormula>,
    before: DurableBuildState,
    content_revision: &str,
    experience: ExactI64,
    lost: ExactI64,
) -> Result<Option<CommittedDeathBuildLoss>> {
    let Some(formula) = formula else {
        // Death never changes trained build state, including for legacy callers
        // without a current training formula (D58 / NIGHT-W1 decision 19).
        return Ok(None);
    };
    if formula.content_revision() != content_revision {
        return Err(CharacterProgressionError::ProgressionContextMismatch);
    }
    let families = calculate_death_build_family_loss(
        build_families(&before),
        experience,
        lost,
        |family, level| formula.required(before.vocation(), family, level),
    )
    .map_err(|_| CharacterProgressionError::BuildStateMismatch)?;
    let mut skills = [(0, 0); 7];
    skills.copy_from_slice(&families[1..]);
    let after = DurableBuildState::new(before.vocation(), families[0], skills)?;
    Ok((before != after).then(|| CommittedDeathBuildLoss {
        before,
        after,
        formula_digest: formula.digest(),
    }))
}

/// Version 2 stores the formula digest next to the request/build digest so
/// reconciliation can recover provenance without a new schema column. The hash
/// binds the exact v1 intent digest and every before/after family value. Version 1
/// is byte-for-byte unchanged for NULL build fields, including after deployment.
fn build_command_binding(legacy: &[u8], build: &CommittedDeathBuildLoss) -> Vec<u8> {
    let mut semantic = Vec::new();
    semantic.extend_from_slice(DEATH_BINDING_DOMAIN);
    semantic.push(DEATH_BUILD_BINDING_VERSION);
    semantic.extend_from_slice(legacy);
    semantic.extend_from_slice(&build.formula_digest);
    for state in [&build.before, &build.after] {
        // The same bounded vocation/family encoding as native build bindings.
        semantic.push(u8::try_from(state.vocation().len()).unwrap_or(u8::MAX));
        semantic.extend_from_slice(state.vocation().as_bytes());
        for (level, progress) in build_families(state) {
            semantic.extend_from_slice(&level.to_be_bytes());
            semantic.extend_from_slice(&progress.to_be_bytes());
        }
    }
    let mut binding = vec![DEATH_BUILD_BINDING_VERSION];
    binding.extend_from_slice(&build.formula_digest);
    binding.extend_from_slice(&Sha256::digest(semantic));
    binding
}

fn decode_build_state(
    row: &sqlx::postgres::PgRow,
    vocation: Option<&str>,
    suffix: &str,
) -> std::result::Result<Option<DurableBuildState>, DurabilityError> {
    let mut families = [(0, 0); 8];
    for (family, value) in families.iter_mut().enumerate() {
        let (level, progress) = if family == 0 {
            (
                format!("magic_level{suffix}"),
                format!("mana_spent{suffix}"),
            )
        } else {
            (
                format!("{}_level{suffix}", SKILLS[family - 1]),
                format!("{}_tries{suffix}", SKILLS[family - 1]),
            )
        };
        let level: Option<i32> = row.try_get(level.as_str())?;
        let progress: Option<i64> = row.try_get(progress.as_str())?;
        match (vocation, level, progress) {
            (Some(_), Some(level), Some(progress)) => {
                *value = (
                    u16::try_from(level).map_err(|_| DurabilityError::InvalidStoredState)?,
                    u64::try_from(progress).map_err(|_| DurabilityError::InvalidStoredState)?,
                );
            }
            (None, None, None) => {}
            _ => return Err(DurabilityError::InvalidStoredState),
        }
    }
    let Some(vocation) = vocation else {
        return Ok(None);
    };
    let mut skills = [(0, 0); 7];
    skills.copy_from_slice(&families[1..]);
    DurableBuildState::new(vocation, families[0], skills)
        .map(Some)
        .map_err(|_| DurabilityError::InvalidStoredState)
}

fn decode_death_build(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<Option<CommittedDeathBuildLoss>, DurabilityError> {
    let vocation: Option<String> = row.try_get("vocation")?;
    let before = decode_build_state(row, vocation.as_deref(), "_before")?;
    let after = decode_build_state(row, vocation.as_deref(), "_after")?;
    let binding: Vec<u8> = row.try_get("command_binding")?;
    match (before, after) {
        (None, None) if binding.len() == 33 && binding[0] == DEATH_BINDING_VERSION => Ok(None),
        (Some(before), Some(after))
            if binding.len() == 65 && binding[0] == DEATH_BUILD_BINDING_VERSION =>
        {
            let before_values = build_families(&before);
            let after_values = build_families(&after);
            if !after_values
                .iter()
                .zip(before_values)
                .all(|(after, before)| *after <= before)
                || after_values == before_values
            {
                return Err(DurabilityError::InvalidStoredState);
            }
            let mut formula_digest = [0; 32];
            formula_digest.copy_from_slice(&binding[1..33]);
            Ok(Some(CommittedDeathBuildLoss {
                before,
                after,
                formula_digest,
            }))
        }
        _ => Err(DurabilityError::InvalidStoredState),
    }
}

pub(super) async fn load_death_build(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
) -> std::result::Result<DurableBuildState, DurabilityError> {
    let row = sqlx::query(
        "SELECT vocation, magic_level, mana_spent, fist_level, fist_tries, \
         club_level, club_tries, sword_level, sword_tries, axe_level, axe_tries, \
         distance_level, distance_tries, shielding_level, shielding_tries, \
         fishing_level, fishing_tries FROM game_character_build_state \
         WHERE character_id = encode($1,'hex')::uuid FOR UPDATE",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(row) = row else {
        return Ok(DurableBuildState::default());
    };
    let vocation: String = row.try_get("vocation")?;
    decode_build_state(&row, Some(&vocation), "")?.ok_or(DurabilityError::InvalidStoredState)
}

fn bind_death_build<'q>(
    mut query: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    build: Option<&CommittedDeathBuildLoss>,
) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
    query = query.bind(build.map(|build| build.before.vocation().to_owned()));
    for state in [
        build.map(|build| &build.before),
        build.map(|build| &build.after),
    ] {
        let values = state.map(build_families);
        for family in 0..8 {
            query = query
                .bind(values.map(|values| i32::from(values[family].0)))
                .bind(values.map(|values| i64::try_from(values[family].1).unwrap_or(i64::MAX)));
        }
    }
    query
}

async fn update_death_build(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    occurrence: PlayerDeathOccurrence,
    revision: u64,
    build: &CommittedDeathBuildLoss,
) -> std::result::Result<(), DurabilityError> {
    let mut query = sqlx::query(
        "UPDATE game_character_build_state SET \
         committed_character_revision=$2::text::numeric(20,0), \
         last_build_occurrence_id=encode($3,'hex')::uuid, \
         magic_level=$4, mana_spent=$5, fist_level=$6, fist_tries=$7, \
         club_level=$8, club_tries=$9, sword_level=$10, sword_tries=$11, \
         axe_level=$12, axe_tries=$13, distance_level=$14, distance_tries=$15, \
         shielding_level=$16, shielding_tries=$17, fishing_level=$18, fishing_tries=$19 \
         WHERE character_id=encode($1,'hex')::uuid AND vocation=$20",
    )
    .bind(character.as_bytes().as_slice())
    .bind(revision.to_string())
    .bind(occurrence.0.to_vec());
    for (level, progress) in build_families(&build.after) {
        query = query
            .bind(i32::from(level))
            .bind(i64::try_from(progress).map_err(|_| DurabilityError::InvalidStoredState)?);
    }
    let updated = query
        .bind(build.before.vocation().to_owned())
        .execute(&mut **tx)
        .await?;
    if updated.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}

async fn load_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    occurrence: PlayerDeathOccurrence,
) -> std::result::Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(sqlx::query(
        "SELECT death_occurrence_id::text, command_binding, character_id::text, \
                original_character_revision::text, committed_character_revision::text, \
                level_before, level_after, experience_before, experience_after, \
                experience_lost, blessings_before, blessings_after, death_world_id::text, \
                death_channel_id::text, death_spatial_position, death_map_revision, \
                respawn_position, vocation, magic_level_before, mana_spent_before, fist_level_before, fist_tries_before, club_level_before, club_tries_before, sword_level_before, sword_tries_before, axe_level_before, axe_tries_before, distance_level_before, distance_tries_before, shielding_level_before, shielding_tries_before, fishing_level_before, fishing_tries_before, magic_level_after, mana_spent_after, fist_level_after, fist_tries_after, club_level_after, club_tries_after, sword_level_after, sword_tries_after, axe_level_after, axe_tries_after, distance_level_after, distance_tries_after, shielding_level_after, shielding_tries_after, fishing_level_after, fishing_tries_after \
           FROM game_character_death_receipts \
          WHERE death_occurrence_id = encode($1,'hex')::uuid",
    )
    .bind(occurrence.0.as_slice())
    .fetch_optional(&mut **tx)
    .await?)
}

fn decode_receipt(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<CommittedCharacterDeath, DurabilityError> {
    let invalid = |_| DurabilityError::InvalidStoredState;
    let level = |column: &str| -> std::result::Result<u32, DurabilityError> {
        u32::try_from(row.try_get::<i64, _>(column)?)
            .map_err(|_| DurabilityError::InvalidStoredState)
    };
    Ok(CommittedCharacterDeath {
        occurrence: PlayerDeathOccurrence(uuid_text(row.try_get("death_occurrence_id")?)?),
        character_id: CharacterId::from_bytes(uuid_text(row.try_get("character_id")?)?)
            .map_err(invalid)?,
        original_character_revision: CharacterRevision::new(numeric_u64(
            row,
            "original_character_revision",
        )?)
        .map_err(invalid)?,
        committed_character_revision: CharacterRevision::new(numeric_u64(
            row,
            "committed_character_revision",
        )?)
        .map_err(invalid)?,
        level_before: level("level_before")?,
        level_after: level("level_after")?,
        experience_before: ExactI64::new(row.try_get("experience_before")?),
        experience_after: ExactI64::new(row.try_get("experience_after")?),
        experience_lost: ExactI64::new(row.try_get("experience_lost")?),
        blessings_before: row.try_get("blessings_before")?,
        blessings_after: row.try_get("blessings_after")?,
        death_cell: DeathCell {
            world_id: WorldId::decode(&uuid_text(row.try_get("death_world_id")?)?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            channel_id: ChannelId::decode(&uuid_text(row.try_get("death_channel_id")?)?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            spatial_position: row.try_get("death_spatial_position")?,
            map_revision: row.try_get("death_map_revision")?,
        },
        respawn_position: row.try_get("respawn_position")?,
        build_loss: decode_death_build(row)?,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::domain::progression::LevelThreshold;
    use crate::foundation::{ConnectionGeneration, GameSessionId, ScopeOwnershipGeneration};
    use oteryn_simulation_determinism::RoundingMode;

    /// The DEATH-2 consumption and settlement are linked in every crate that compiles this
    /// module, as the writer is in `durability::character_progression_linkage`.
    #[test]
    fn respawn_consumption_api_is_linked() {
        let _ = std::mem::size_of::<ConsumedRespawn>();
        let _ = DurabilityRoot::consume_pending_respawn;
        let _ = DurabilityRoot::pending_respawn;
        let _ = DurabilityRoot::settle_player_death::<2>;
    }

    fn id(seed: u8) -> [u8; 16] {
        [
            seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
        ]
    }

    fn context() -> ProgressionRevisionContext<String> {
        ProgressionRevisionContext {
            profile: "profile-1".into(),
            ruleset: "ruleset-1".into(),
            content: "content-1".into(),
            simulation: "simulation-1".into(),
            evidence: "evidence-1".into(),
            declaration: "declaration-1".into(),
        }
    }

    fn request() -> CharacterDeathRequest<2> {
        let context = context();
        CharacterDeathRequest {
            pvp_contributors: Vec::new(),
            occurrence: PlayerDeathOccurrence::from_bytes(id(1)).expect("occurrence"),
            context: context.clone(),
            policy_revision: "policy-1".into(),
            reward_revision: "reward-1".into(),
            policy: FiniteProgressionPolicy {
                context,
                policy_revision: "policy-1".into(),
                reward_revision: "reward-1".into(),
                death_policy_revision: "death-1".into(),
                declared_difference_revision: "declaration-1".into(),
                thresholds: [
                    LevelThreshold {
                        level: 1,
                        minimum_experience: ExactI64::new(0),
                    },
                    LevelThreshold {
                        level: 2,
                        minimum_experience: ExactI64::new(100),
                    },
                ],
                terminal_exclusive_experience: ExactI64::new(200),
                death_loss_numerator: 1,
                death_loss_denominator: 1,
                death_loss_rounding: RoundingMode::Floor,
            },
            held_blessings: vec!["embrace".into(), "spark".into()],
            death_cell: DeathCell {
                world_id: WorldId::decode(&id(4)).expect("world"),
                channel_id: ChannelId::decode(&id(5)).expect("channel"),
                spatial_position: vec![1, 2, 3],
                map_revision: "map-1".into(),
            },
            respawn_position: b"temple:thais".to_vec(),
        }
    }

    fn fence() -> CurrentCharacterGameplayFence {
        CurrentCharacterGameplayFence {
            character_id: CharacterId::from_bytes(id(2)).expect("character"),
            game_session_id: GameSessionId::decode(&id(3)).expect("session"),
            connection_generation: ConnectionGeneration::new(1).expect("connection"),
            character_lease_generation: 1,
            runtime_scope: RuntimeScopeRefV1::channel(
                WorldId::decode(&id(4)).expect("world"),
                ChannelId::decode(&id(5)).expect("channel"),
            ),
            scope_ownership_generation: ScopeOwnershipGeneration::new(1).expect("scope"),
            expected_character_revision: CharacterRevision::new(2).expect("revision"),
        }
    }

    #[test]
    fn death_binding_covers_the_intent_and_excludes_retry_local_authority() {
        let death = request();
        let binding = command_binding(&fence(), &death).expect("binding");
        assert_eq!(binding.len(), 33);

        let mut reconnected = fence();
        reconnected.game_session_id = GameSessionId::decode(&id(9)).expect("session");
        reconnected.connection_generation = ConnectionGeneration::new(7).expect("connection");
        reconnected.character_lease_generation = 8;
        reconnected.scope_ownership_generation = ScopeOwnershipGeneration::new(9).expect("scope");
        assert_eq!(
            command_binding(&reconnected, &death).expect("replay"),
            binding
        );

        let mut changed: Vec<CharacterDeathRequest<2>> = Vec::new();
        let mut blessings = death.clone();
        blessings.held_blessings.pop();
        changed.push(blessings);
        let mut cell = death.clone();
        cell.death_cell.spatial_position = vec![1, 2, 4];
        changed.push(cell);
        let mut map = death.clone();
        map.death_cell.map_revision = "map-2".into();
        changed.push(map);
        let mut respawn = death.clone();
        respawn.respawn_position = b"temple:carlin".to_vec();
        changed.push(respawn);
        let mut policy = death.clone();
        policy.policy.death_loss_numerator = 2;
        changed.push(policy);
        let mut death_policy = death.clone();
        death_policy.policy.death_policy_revision = "death-2".into();
        changed.push(death_policy);
        for other in changed {
            assert_ne!(command_binding(&fence(), &other).expect("changed"), binding);
        }
        let mut revision = fence();
        revision.expected_character_revision = CharacterRevision::new(3).expect("revision");
        assert_ne!(
            command_binding(&revision, &death).expect("revision"),
            binding
        );
    }

    #[test]
    fn invalid_death_input_fails_before_database_work() {
        assert!(matches!(
            PlayerDeathOccurrence::from_bytes([0; 16]),
            Err(CharacterProgressionError::InvalidInput)
        ));
        validate_request(&fence(), &request()).expect("valid request");
        let mut invalid: Vec<CharacterDeathRequest<2>> = Vec::new();
        let mut unsorted = request();
        unsorted.held_blessings.reverse();
        invalid.push(unsorted);
        let mut duplicate = request();
        duplicate.held_blessings = vec!["spark".into(), "spark".into()];
        invalid.push(duplicate);
        let mut malformed = request();
        malformed.held_blessings = vec!["-spark".into()];
        invalid.push(malformed);
        let mut too_many = request();
        too_many.held_blessings = (0..33).map(|index| format!("b{index:02}")).collect();
        invalid.push(too_many);
        let mut empty_cell = request();
        empty_cell.death_cell.spatial_position.clear();
        invalid.push(empty_cell);
        let mut long_cell = request();
        long_cell.death_cell.spatial_position = vec![1; 129];
        invalid.push(long_cell);
        let mut empty_map = request();
        empty_map.death_cell.map_revision.clear();
        invalid.push(empty_map);
        let mut empty_respawn = request();
        empty_respawn.respawn_position.clear();
        invalid.push(empty_respawn);
        let mut unbound = request();
        unbound.context.content = "content-2".into();
        invalid.push(unbound);
        for death in invalid {
            assert!(matches!(
                validate_request(&fence(), &death),
                Err(CharacterProgressionError::InvalidInput)
            ));
        }
        let mut unleased = fence();
        unleased.character_lease_generation = 0;
        assert!(matches!(
            validate_request(&unleased, &request()),
            Err(CharacterProgressionError::InvalidInput)
        ));
    }
}

// Deterministic policy/control-flow tests. No callback below represents a DB fence.
#[cfg(test)]
#[allow(clippy::expect_used)]
mod death_training_flush_tests {
    use super::*;
    use crate::durability::character_build::{BuildOccurrence, DurableBuildState};
    use crate::foundation::{ConnectionGeneration, GameSessionId, ScopeOwnershipGeneration};
    use std::cell::RefCell;
    use std::collections::VecDeque;

    fn id(seed: u8) -> [u8; 16] {
        let mut id = [seed; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id
    }
    fn revision(value: u64) -> CharacterRevision {
        CharacterRevision::new(value).expect("revision")
    }
    fn fence() -> CurrentCharacterGameplayFence {
        CurrentCharacterGameplayFence {
            character_id: CharacterId::from_bytes(id(1)).expect("character"),
            game_session_id: GameSessionId::decode(&id(2)).expect("session"),
            connection_generation: ConnectionGeneration::new(1).expect("connection"),
            character_lease_generation: 1,
            runtime_scope: RuntimeScopeRefV1::channel(
                WorldId::decode(&id(3)).expect("world"),
                ChannelId::decode(&id(4)).expect("channel"),
            ),
            scope_ownership_generation: ScopeOwnershipGeneration::new(1).expect("scope"),
            expected_character_revision: revision(2),
        }
    }
    fn request() -> BuildChangeRequest {
        BuildChangeRequest {
            occurrence: BuildOccurrence::from_bytes(id(5)).expect("occurrence"),
            cause: BuildCause::Training,
            before: DurableBuildState::default(),
            after: DurableBuildState::new("none", (0, 7), [(10, 0); 7]).expect("after"),
            pruned_stance: None,
        }
    }
    fn receipt() -> CommittedBuildChange {
        let request = request();
        CommittedBuildChange {
            occurrence: request.occurrence,
            character_id: fence().character_id,
            original_character_revision: revision(2),
            committed_character_revision: revision(3),
            cause: request.cause,
            before: request.before,
            after: request.after,
            pruned_stance: None,
        }
    }
    struct Formula;
    impl BuildFormula for Formula {
        fn required(&self, _: &str, _: usize, _: u16) -> Option<u64> {
            Some(100)
        }
        fn digest(&self) -> [u8; 32] {
            [7; 32]
        }
        fn content_revision(&self) -> &str {
            "content-1"
        }
    }
    fn ready<T>(future: impl std::future::Future<Output = T>) -> T {
        let mut future = std::pin::pin!(future);
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        let result = match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(value) => Some(value),
            std::task::Poll::Pending => None,
        };
        result.expect("deterministic callbacks must be immediately ready")
    }
    fn unknown() -> CharacterProgressionError {
        CharacterProgressionError::Unavailable(DurabilityError::InvalidStoredState)
    }
    async fn run(
        writes: Vec<Result<BuildCommitOutcome>>,
        reads: Vec<Result<Option<CommittedBuildChange>>>,
    ) -> (Result<CommittedBuildChange>, Vec<String>) {
        let events = RefCell::new(Vec::new());
        let mut writes = VecDeque::from(writes);
        let mut reads = VecDeque::from(reads);
        let result = settle_death_training_flush(
            fence().character_id,
            revision(2),
            &request(),
            async |original| {
                events.borrow_mut().push(format!(
                    "write:{}",
                    original.expect("prepared revision").get()
                ));
                writes.pop_front().expect("unexpected write")
            },
            async || {
                events.borrow_mut().push("reconcile".into());
                reads.pop_front().expect("unexpected reconcile")
            },
        )
        .await;
        (result, events.into_inner())
    }
    #[test]
    fn checkpoint_identity_changes_are_refused_independently() {
        let base = fence();
        let request = request();
        let mut mutations = Vec::new();
        let mut f = base;
        f.character_id = CharacterId::from_bytes(id(9)).expect("character");
        mutations.push(f);
        let mut f = base;
        f.game_session_id = GameSessionId::decode(&id(9)).expect("session");
        mutations.push(f);
        let mut f = base;
        f.connection_generation = ConnectionGeneration::new(2).expect("connection");
        mutations.push(f);
        let mut f = base;
        f.character_lease_generation = 2;
        mutations.push(f);
        let mut f = base;
        f.runtime_scope = RuntimeScopeRefV1::channel(
            WorldId::decode(&id(9)).expect("world"),
            ChannelId::decode(&id(4)).expect("channel"),
        );
        mutations.push(f);
        let mut f = base;
        f.scope_ownership_generation = ScopeOwnershipGeneration::new(2).expect("scope");
        mutations.push(f);
        for changed in mutations {
            assert!(matches!(
                validate_death_training_input(
                    base,
                    &DeathTrainingFlush {
                        fence: changed,
                        request: &request,
                        formula: &Formula,
                        formula_digest: [7; 32]
                    }
                ),
                Err(CharacterProgressionError::AuthorityRejected)
            ));
        }
        let mut old_revision = base;
        old_revision.expected_character_revision = revision(1);
        validate_death_training_input(
            base,
            &DeathTrainingFlush {
                fence: old_revision,
                request: &request,
                formula: &Formula,
                formula_digest: [7; 32],
            },
        )
        .expect("revision is checked by the sequenced writer, not identity matching");
    }
    #[test]
    fn non_training_prune_and_formula_substitution_are_rejected() {
        for index in 0..3 {
            let mut request = request();
            let mut digest = [7; 32];
            match index {
                0 => request.cause = BuildCause::Promotion,
                1 => request.pruned_stance = Some("stance".into()),
                _ => digest = [8; 32],
            }
            assert!(matches!(
                validate_death_training_input(
                    fence(),
                    &DeathTrainingFlush {
                        fence: fence(),
                        request: &request,
                        formula: &Formula,
                        formula_digest: digest
                    }
                ),
                Err(CharacterProgressionError::InvalidInput)
            ));
        }
    }
    #[test]
    fn historical_receipt_mutations_do_not_match_the_intended_checkpoint() {
        let original = receipt();
        let mut mutations = Vec::new();
        let mut r = original.clone();
        r.character_id = CharacterId::from_bytes(id(9)).expect("character");
        mutations.push(r);
        let mut r = original.clone();
        r.occurrence = BuildOccurrence::from_bytes(id(9)).expect("occurrence");
        mutations.push(r);
        let mut r = original.clone();
        r.cause = BuildCause::Promotion;
        mutations.push(r);
        let mut r = original.clone();
        r.before = r.after.clone();
        mutations.push(r);
        let mut r = original.clone();
        r.after = DurableBuildState::default();
        mutations.push(r);
        let mut r = original.clone();
        r.pruned_stance = Some("stance".into());
        mutations.push(r);
        let mut r = original.clone();
        r.original_character_revision = revision(1);
        r.committed_character_revision = revision(2);
        mutations.push(r);
        let mut r = original.clone();
        r.committed_character_revision = revision(4);
        mutations.push(r);
        for changed in mutations {
            assert!(matches!(
                validate_death_training_receipt(
                    fence().character_id,
                    revision(2),
                    &request(),
                    &changed
                ),
                Err(CharacterProgressionError::ConflictingOccurrence)
            ));
        }
        validate_death_training_receipt(fence().character_id, revision(2), &request(), &original)
            .expect("matching history only");
    }
    #[test]
    fn fresh_flush_reconciles_then_commits_at_prepared_revision() {
        ready(async {
            let (result, events) = run(
                vec![Ok(BuildCommitOutcome::Committed(receipt()))],
                vec![Ok(None)],
            )
            .await;
            assert_eq!(
                result.expect("committed").committed_character_revision,
                revision(3)
            );
            assert_eq!(events, ["reconcile", "write:2"]);
        });
    }
    #[test]
    fn retained_flush_is_binding_replayed_once_not_newly_checkpointed() {
        ready(async {
            let (result, events) = run(
                vec![Ok(BuildCommitOutcome::AlreadyCommitted(receipt()))],
                vec![Ok(Some(receipt()))],
            )
            .await;
            result.expect("replayed");
            assert_eq!(events, ["reconcile", "write:2"]);
        });
    }
    #[test]
    fn ambiguous_commit_requires_reconciliation_then_exact_writer_replay() {
        ready(async {
            let (result, events) = run(
                vec![
                    Err(unknown()),
                    Ok(BuildCommitOutcome::AlreadyCommitted(receipt())),
                ],
                vec![Ok(None), Ok(Some(receipt()))],
            )
            .await;
            result.expect("resolved");
            assert_eq!(events, ["reconcile", "write:2", "reconcile", "write:2"]);
        });
    }
    #[test]
    fn unresolved_or_failed_reconcile_never_retries_the_write() {
        ready(async {
            for read in [Ok(None), Err(unknown())] {
                let (result, events) = run(vec![Err(unknown())], vec![Ok(None), read]).await;
                assert!(result.is_err());
                assert_eq!(events, ["reconcile", "write:2", "reconcile"]);
            }
        });
    }
    #[test]
    fn stale_fence_and_definitive_failures_stop_without_rebase() {
        ready(async {
            for error in [
                CharacterProgressionError::CharacterRevisionMismatch,
                CharacterProgressionError::AuthorityRejected,
                CharacterProgressionError::BuildStateMismatch,
                CharacterProgressionError::ConflictingOccurrence,
            ] {
                let (result, events) = run(vec![Err(error)], vec![Ok(None)]).await;
                assert!(result.is_err());
                assert_eq!(events, ["reconcile", "write:2"]);
            }
        });
    }
    #[test]
    fn mismatched_history_is_not_permission_to_commit_or_replay() {
        ready(async {
            let mut bad = receipt();
            bad.original_character_revision = revision(1);
            bad.committed_character_revision = revision(2);
            let (result, events) = run(vec![], vec![Ok(Some(bad))]).await;
            assert!(result.is_err());
            assert_eq!(events, ["reconcile"]);
        });
    }
    #[test]
    fn reconciled_history_still_cannot_override_writer_binding_conflict() {
        ready(async {
            let (result, events) = run(
                vec![
                    Err(unknown()),
                    Err(CharacterProgressionError::ConflictingOccurrence),
                ],
                vec![Ok(None), Ok(Some(receipt()))],
            )
            .await;
            assert!(matches!(
                result,
                Err(CharacterProgressionError::ConflictingOccurrence)
            ));
            assert_eq!(events, ["reconcile", "write:2", "reconcile", "write:2"]);
        });
    }
    #[test]
    fn failed_initial_reconcile_never_attempts_the_write() {
        ready(async {
            let (result, events) = run(vec![], vec![Err(unknown())]).await;
            assert!(result.is_err());
            assert_eq!(events, ["reconcile"]);
        });
    }
    #[test]
    fn successful_writer_with_wrong_committed_revision_is_rejected() {
        ready(async {
            let mut bad = receipt();
            bad.committed_character_revision = revision(4);
            let (result, events) =
                run(vec![Ok(BuildCommitOutcome::Committed(bad))], vec![Ok(None)]).await;
            assert!(result.is_err());
            assert_eq!(events, ["reconcile", "write:2"]);
        });
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "bounded static binding fixtures")]
mod death_build_binding_tests {
    use super::*;

    fn state(vocation: &str, families: [(u16, u64); 8]) -> DurableBuildState {
        let mut skills = [(0, 0); 7];
        skills.copy_from_slice(&families[1..]);
        DurableBuildState::new(vocation, families[0], skills).expect("bounded fixture")
    }

    #[test]
    fn version_two_binding_covers_every_before_after_family_and_formula() {
        let mut before = [(12, 50); 8];
        before[0] = (2, 50);
        let mut after = [(11, 20); 8];
        after[0] = (1, 20);
        let build = CommittedDeathBuildLoss {
            before: state("none", before),
            after: state("none", after),
            formula_digest: [7; 32],
        };
        let legacy = [1; 33];
        let expected = build_command_binding(&legacy, &build);
        assert_eq!(expected.len(), 65);
        assert_eq!(expected[0], 2);
        assert_eq!(expected[1..33], build.formula_digest);
        for family in 0..8 {
            let mut changed = build.clone();
            let mut values = before;
            values[family].1 += 1;
            changed.before = state("none", values);
            assert_ne!(build_command_binding(&legacy, &changed), expected);
            let mut changed = build.clone();
            let mut values = after;
            values[family].1 += 1;
            changed.after = state("none", values);
            assert_ne!(build_command_binding(&legacy, &changed), expected);
        }
        let mut changed = build.clone();
        changed.formula_digest[0] ^= 1;
        assert_ne!(build_command_binding(&legacy, &changed), expected);
        let mut changed = build.clone();
        changed.before = state("knight", before);
        changed.after = state("knight", after);
        assert_ne!(build_command_binding(&legacy, &changed), expected);
        let mut other_intent = legacy;
        other_intent[1] ^= 1;
        assert_ne!(build_command_binding(&other_intent, &build), expected);
    }
}

#[cfg(test)]
mod death_build_postgres {
    use super::*;
    use crate::domain::progression::LevelThreshold;
    use crate::foundation::{ConnectionGeneration, GameSessionId, ScopeOwnershipGeneration};
    use oteryn_simulation_determinism::RoundingMode;
    use std::future::{Future, poll_fn};
    use std::task::Poll;

    // Real shared native harness; local mount also works when another approved
    // PostgreSQL target compiles the durability source into its own crate.
    // Many targets that mount this source have no root harness, so the copy
    // stays local. Its nested `ground_native_readiness_enforcement` file is
    // then mounted a second time in crates that also mount the harness at root.
    #[allow(
        dead_code,
        clippy::duplicate_mod,
        reason = "shared real PostgreSQL harness; this mount uses a subset and \
                  may coexist with a root mount of the same support file"
    )]
    mod harness {
        include!("../../../../../apps/game-server/tests/support/bestiary_postgres_harness.rs");
    }
    use harness::{Harness, TestResult, debug, fence, id};

    struct Table([u8; 32], &'static str);
    impl BuildFormula for Table {
        fn required(&self, _: &str, _: usize, _: u16) -> Option<u64> {
            Some(100)
        }
        fn digest(&self) -> [u8; 32] {
            self.0
        }
        fn content_revision(&self) -> &str {
            self.1
        }
    }
    const TABLE: Table = Table([17; 32], "content-1");

    fn run<F>(tag: &'static str, body: F) -> TestResult
    where
        F: AsyncFnOnce(&Harness) -> TestResult,
    {
        let Some(admin) = harness::configured_admin() else {
            return Ok(());
        };
        harness::runtime()?.block_on(async move {
            let h = Harness::create(admin, tag, true).await?;
            let result = body(&h).await;
            h.cleanup().await?;
            result
        })
    }

    fn request(tag: u8) -> TestResult<CharacterDeathRequest<50>> {
        // Constraint fixture: harness initializes level50/1000XP. The native
        // D58 calculator still supplies the actual capped loss; these thresholds
        // only project the fixture's before/after levels.
        let thresholds = std::array::from_fn(|index| LevelThreshold {
            level: u32::try_from(index + 1).unwrap_or(u32::MAX),
            minimum_experience: ExactI64::new(i64::try_from(index).unwrap_or(i64::MAX) * 20),
        });
        let context = harness::context();
        let policy = FiniteProgressionPolicy {
            context: context.clone(),
            policy_revision: "policy-1".into(),
            reward_revision: "reward-1".into(),
            death_policy_revision: "death-1".into(),
            declared_difference_revision: "declaration-1".into(),
            thresholds,
            terminal_exclusive_experience: ExactI64::new(1_000_000),
            death_loss_numerator: 1,
            death_loss_denominator: 1,
            death_loss_rounding: RoundingMode::Floor,
        };
        Ok(CharacterDeathRequest {
            pvp_contributors: Vec::new(),
            occurrence: PlayerDeathOccurrence::from_bytes(id(tag)).map_err(debug)?,
            context,
            policy_revision: "policy-1".into(),
            reward_revision: "reward-1".into(),
            policy,
            held_blessings: vec![],
            death_cell: DeathCell {
                world_id: WorldId::decode(&id(harness::WORLD)).map_err(debug)?,
                channel_id: ChannelId::decode(&id(harness::CHANNEL)).map_err(debug)?,
                spatial_position: vec![1, 2, 3, 7],
                map_revision: "map-1".into(),
            },
            respawn_position: b"temple:thais".to_vec(),
        })
    }

    fn prepared_training() -> TestResult<BuildChangeRequest> {
        Ok(BuildChangeRequest {
            occurrence: super::super::character_build::BuildOccurrence::from_bytes(id(70))
                .map_err(debug)?,
            cause: BuildCause::Training,
            before: DurableBuildState::default(),
            after: DurableBuildState::new("none", (2, 50), [(12, 50); 7]).map_err(debug)?,
            pruned_stance: None,
        })
    }

    async fn train(h: &Harness) -> TestResult<DurableBuildState> {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let training = prepared_training()?;
        let after = training.after.clone();
        let outcome = h
            .root
            .commit_character_build(&authority, &h.node, fence(1)?, training, &TABLE)
            .await
            .map_err(debug)?;
        assert!(matches!(outcome, BuildCommitOutcome::Committed(_)));
        Ok(after)
    }

    async fn snapshot(h: &Harness) -> TestResult<String> {
        Ok(sqlx::query_scalar(
            "SELECT jsonb_build_object(\
             'roots',(SELECT jsonb_agg(to_jsonb(r) ORDER BY character_id) FROM game_character_roots r),\
             'progression',(SELECT jsonb_agg(to_jsonb(r) ORDER BY character_id) FROM game_character_progression_state r),\
             'build',(SELECT jsonb_agg(to_jsonb(r) ORDER BY character_id) FROM game_character_build_state r),\
             'build_receipts',(SELECT jsonb_agg(to_jsonb(r) ORDER BY build_occurrence_id) FROM game_character_build_receipts r),\
             'deaths',(SELECT jsonb_agg(to_jsonb(r) ORDER BY death_occurrence_id) FROM game_character_death_receipts r),\
             'blessings',(SELECT jsonb_agg(to_jsonb(r) ORDER BY character_id,blessing_key) FROM game_character_blessings r),\
             'respawns',(SELECT jsonb_agg(to_jsonb(r) ORDER BY character_id) FROM game_character_pending_respawns r))::text"
        ).fetch_one(&h.pool).await?)
    }

    #[test]
    fn historical_v2_training_then_death_replays_and_reconciles_after_restart() -> TestResult {
        run("death_build_restart", async |h| {
            let pending = prepared_training()?;
            let before = pending.after.clone();
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let death = request(71)?;
            let committed = h
                .root
                .settle_player_death_with_training_and_build(
                    &CharacterRevisionSequencer::new(),
                    &authority,
                    &h.node,
                    fence(1)?,
                    death.clone(),
                    Some(DeathTrainingFlush {
                        fence: fence(1)?,
                        request: &pending,
                        formula: &TABLE,
                        formula_digest: TABLE.digest(),
                    }),
                    &TABLE,
                )
                .await
                .map_err(debug)?;
            assert!(committed.build_loss.is_none());
            // Stored pre-decision history: the old writer took all progress when
            // capped XP loss was 100%. Plant its exact v2 receipt/build row, then
            // exercise the real restart decoder, verifier and replay APIs below.
            let mut committed = committed;
            let historical = CommittedDeathBuildLoss {
                before: before.clone(),
                after: DurableBuildState::default(),
                formula_digest: TABLE.digest(),
            };
            let binding = build_command_binding(
                &command_binding(&fence(2)?, &death).map_err(debug)?,
                &historical,
            );
            let mut receipt_fields = vec![
                "vocation = 'none'".to_owned(),
                "stamina_minutes_before = 2520".to_owned(),
                "stamina_minutes_after = 2520".to_owned(),
                "offline_pool_s_before = 43200".to_owned(),
                "offline_pool_s_after = 43200".to_owned(),
                "offline_skill_before = -1".to_owned(),
                "offline_skill_after = -1".to_owned(),
            ];
            let mut state_fields = vec![
                "magic_level = 0".to_owned(),
                "mana_spent = 0".to_owned(),
                "committed_character_revision = 3".to_owned(),
                "last_build_occurrence_id = encode($1,'hex')::uuid".to_owned(),
            ];
            for (suffix, state) in [("before", &historical.before), ("after", &historical.after)] {
                for (family, (level, progress)) in build_families(state).into_iter().enumerate() {
                    let (level_column, progress_column) = if family == 0 {
                        ("magic_level".to_owned(), "mana_spent".to_owned())
                    } else {
                        (
                            format!("{}_level", SKILLS[family - 1]),
                            format!("{}_tries", SKILLS[family - 1]),
                        )
                    };
                    receipt_fields.push(format!("{level_column}_{suffix} = {level}"));
                    receipt_fields.push(format!("{progress_column}_{suffix} = {progress}"));
                    if suffix == "after" && family != 0 {
                        state_fields.push(format!("{level_column} = {level}"));
                        state_fields.push(format!("{progress_column} = {progress}"));
                    }
                }
            }
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role = replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "UPDATE game_character_death_receipts SET {}, command_binding = $1 WHERE death_occurrence_id = encode($2,'hex')::uuid",
                receipt_fields.join(",")
            )))
            .bind(binding)
            .bind(death.occurrence.0.as_slice())
            .execute(&mut *tx)
            .await?;
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "UPDATE game_character_build_state SET {} WHERE character_id = encode($2,'hex')::uuid",
                state_fields.join(",")
            )))
            .bind(death.occurrence.0.as_slice())
            .bind(fence(2)?.character_id.as_bytes().as_slice())
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            committed.build_loss = Some(historical);
            let build = committed
                .build_loss
                .as_ref()
                .ok_or("missing build payload")?;
            assert_eq!(build.before, before);
            assert_eq!(build.after, DurableBuildState::default());
            assert_eq!(build.formula_digest, TABLE.digest());
            assert_eq!(committed.experience_lost.get(), 1000);
            assert_eq!(h.count("game_character_build_receipts").await?, 1);
            assert_eq!(h.count("game_character_death_receipts").await?, 1);
            assert_eq!(h.root_revision().await?, "3");
            let stored: (String, String) = sqlx::query_as(
                "SELECT committed_character_revision::text, last_build_occurrence_id::text FROM game_character_build_state"
            ).fetch_one(&h.pool).await?;
            assert_eq!(stored.0, "3");
            let receipt_occurrence: String = sqlx::query_scalar(
                "SELECT death_occurrence_id::text FROM game_character_death_receipts",
            )
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(stored.1, receipt_occurrence);
            let binding: Vec<u8> =
                sqlx::query_scalar("SELECT command_binding FROM game_character_death_receipts")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(binding.len(), 65);
            assert_eq!(binding[0], DEATH_BUILD_BINDING_VERSION);
            let unchanged = snapshot(h).await?;
            let restarted = DurabilityRoot::connect_test_runtime(&h.database.url)?;
            assert!(restarted.maintain_ready_once().await?);
            let reopened = restarted
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            assert_eq!(
                restarted
                    .reconcile_character_death(&reopened, death.occurrence)
                    .await
                    .map_err(debug)?,
                Some(committed.clone())
            );
            let replay = restarted
                .commit_character_death_with_build(
                    &reopened,
                    &h.node,
                    fence(2)?,
                    death.clone(),
                    &TABLE,
                )
                .await
                .map_err(debug)?;
            assert_eq!(
                replay,
                CharacterDeathOutcome::AlreadyCommitted(committed.clone())
            );
            // Retained death replay skips even an invalid newly supplied live
            // checkpoint; it must not validate/capture/apply it over historical state.
            let mut ignored_training_fence = fence(2)?;
            ignored_training_fence.character_lease_generation = 99;
            assert_eq!(
                restarted
                    .settle_player_death_with_training_and_build(
                        &CharacterRevisionSequencer::new(),
                        &reopened,
                        &h.node,
                        fence(2)?,
                        death.clone(),
                        Some(DeathTrainingFlush {
                            fence: ignored_training_fence,
                            request: &pending,
                            formula: &TABLE,
                            formula_digest: [99; 32],
                        }),
                        &Table([99; 32], "different-current-content"),
                    )
                    .await
                    .map_err(debug)?,
                committed,
            );
            // Historical replay also remains available to the compatibility caller.
            assert_eq!(
                restarted
                    .commit_character_death(&reopened, &h.node, fence(2)?, death.clone())
                    .await
                    .map_err(debug)?,
                CharacterDeathOutcome::AlreadyCommitted(committed)
            );
            let mut changed = death.clone();
            changed.respawn_position = b"temple:carlin".to_vec();
            assert!(matches!(
                restarted
                    .commit_character_death_with_build(
                        &reopened,
                        &h.node,
                        fence(2)?,
                        changed,
                        &TABLE
                    )
                    .await,
                Err(CharacterProgressionError::ConflictingOccurrence)
            ));
            assert!(matches!(
                restarted
                    .commit_character_death_with_build(
                        &reopened,
                        &h.node,
                        fence(2)?,
                        death,
                        &Table([99; 32], "content-1")
                    )
                    .await,
                Err(CharacterProgressionError::ConflictingOccurrence)
            ));
            assert_eq!(snapshot(h).await?, unchanged);
            Ok(())
        })
    }

    #[test]
    fn native_legacy_null_v1_replay_preserves_exact_binding_across_build_adoption() -> TestResult {
        run("death_build_legacy", async |h| {
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let death = request(72)?;
            let expected = command_binding(&fence(1)?, &death).map_err(debug)?;
            let outcome = h
                .root
                .commit_character_death(&authority, &h.node, fence(1)?, death.clone())
                .await
                .map_err(debug)?;
            let CharacterDeathOutcome::Committed(committed) = outcome else {
                return Err("expected legacy death".into());
            };
            assert!(committed.build_loss.is_none());
            assert_eq!(h.count("game_character_build_state").await?, 0);
            let binding: Vec<u8> =
                sqlx::query_scalar("SELECT command_binding FROM game_character_death_receipts")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(binding, expected);
            assert_eq!(binding.len(), 33);
            assert_eq!(binding[0], 1);
            let before = snapshot(h).await?;
            // The newly supplied current formula is irrelevant to a retained v1 death.
            let replay = h
                .root
                .commit_character_death_with_build(
                    &authority,
                    &h.node,
                    fence(1)?,
                    death,
                    &Table([99; 32], "different-current-content"),
                )
                .await
                .map_err(debug)?;
            assert_eq!(replay, CharacterDeathOutcome::AlreadyCommitted(committed));
            assert_eq!(snapshot(h).await?, before);
            Ok(())
        })
    }

    #[test]
    fn native_death_build_write_and_receipt_failure_roll_back_together() -> TestResult {
        run("death_build_rollback", async |h| {
            train(h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let before = snapshot(h).await?;
            // Failure occurs after the native root/progression/build UPDATEs, at
            // receipt insertion. This fixture never substitutes an owner writer.
            sqlx::query("CREATE FUNCTION death_build_test_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'death build rollback probe'; END $$")
                .execute(&h.pool).await?;
            sqlx::query("CREATE TRIGGER death_build_test_fail BEFORE INSERT ON game_character_death_receipts FOR EACH ROW EXECUTE FUNCTION death_build_test_fail()")
                .execute(&h.pool).await?;
            let death = request(73)?;
            assert!(
                h.root
                    .commit_character_death_with_build(
                        &authority,
                        &h.node,
                        fence(2)?,
                        death.clone(),
                        &TABLE
                    )
                    .await
                    .is_err()
            );
            assert_eq!(snapshot(h).await?, before);
            assert!(
                h.root
                    .reconcile_character_death(&authority, death.occurrence)
                    .await
                    .map_err(debug)?
                    .is_none()
            );
            sqlx::query("DROP TRIGGER death_build_test_fail ON game_character_death_receipts")
                .execute(&h.pool)
                .await?;
            sqlx::query("DROP FUNCTION death_build_test_fail()")
                .execute(&h.pool)
                .await?;
            assert!(matches!(
                h.root
                    .commit_character_death_with_build(
                        &authority,
                        &h.node,
                        fence(2)?,
                        death,
                        &TABLE
                    )
                    .await
                    .map_err(debug)?,
                CharacterDeathOutcome::Committed(_)
            ));
            assert_eq!(h.count("game_character_build_receipts").await?, 1);
            assert_eq!(h.count("game_character_death_receipts").await?, 1);
            Ok(())
        })
    }

    #[test]
    fn native_death_requires_current_fences_and_content_and_preserves_build() -> TestResult {
        run("death_build_current", async |h| {
            train(h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let before = snapshot(h).await?;
            let mut connection = fence(2)?;
            connection.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
            let mut lease = fence(2)?;
            lease.character_lease_generation = 2;
            let mut scope = fence(2)?;
            scope.scope_ownership_generation = ScopeOwnershipGeneration::new(2).map_err(debug)?;
            let mut session = fence(2)?;
            session.game_session_id = GameSessionId::decode(&id(51)).map_err(debug)?;
            for (index, stale) in [connection, lease, scope, session].into_iter().enumerate() {
                let death = request(80 + u8::try_from(index)?)?;
                assert!(matches!(
                    h.root
                        .commit_character_death_with_build(
                            &authority, &h.node, stale, death, &TABLE
                        )
                        .await,
                    Err(CharacterProgressionError::AuthorityRejected)
                ));
                assert_eq!(snapshot(h).await?, before);
            }
            assert!(matches!(
                h.root
                    .commit_character_death_with_build(
                        &authority,
                        &h.node,
                        fence(1)?,
                        request(85)?,
                        &TABLE
                    )
                    .await,
                Err(CharacterProgressionError::CharacterRevisionMismatch)
            ));
            assert!(matches!(
                h.root
                    .commit_character_death_with_build(
                        &authority,
                        &h.node,
                        fence(2)?,
                        request(86)?,
                        &Table([17; 32], "stale-content")
                    )
                    .await,
                Err(CharacterProgressionError::ProgressionContextMismatch)
            ));
            assert_eq!(snapshot(h).await?, before);
            let outcome = h
                .root
                .commit_character_death(&authority, &h.node, fence(2)?, request(87)?)
                .await
                .map_err(debug)?;
            let CharacterDeathOutcome::Committed(death) = outcome else {
                return Err("legacy death unexpectedly replayed".into());
            };
            assert!(death.build_loss.is_none());
            assert_eq!(death.experience_lost.get(), 1000);
            assert_eq!(
                h.root
                    .read_character_build_state(&authority, fence(3)?.character_id)
                    .await
                    .map_err(debug)?,
                prepared_training()?.after
            );
            Ok(())
        })
    }

    /// Runtime deaths preserve trained builds with current, absent or unavailable
    /// content formulas; replay and fenced respawn consumption still converge.
    #[test]
    fn native_runtime_death_of_trained_character_preserves_build_and_respawns() -> TestResult {
        run("death_build_runtime", async |h| {
            let trained = train(h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let sequencer = CharacterRevisionSequencer::new();
            let death = request(96)?;
            let committed = h
                .root
                .settle_runtime_player_death(
                    &sequencer,
                    &authority,
                    &h.node,
                    fence(2)?,
                    death.clone(),
                    Some(&TABLE),
                    None,
                )
                .await
                .map_err(debug)?;
            assert!(committed.build_loss.is_none());
            assert_eq!(committed.experience_lost.get(), 1000);
            assert_eq!(
                h.root
                    .read_character_build_state(&authority, fence(3)?.character_id)
                    .await
                    .map_err(debug)?,
                trained
            );
            assert_eq!(h.root_revision().await?, "3");
            assert_eq!(h.count("game_character_death_receipts").await?, 1);
            assert_eq!(h.count("game_character_pending_respawns").await?, 1);
            assert_eq!(
                h.root
                    .settle_runtime_player_death(
                        &sequencer,
                        &authority,
                        &h.node,
                        fence(3)?,
                        death.clone(),
                        Some(&TABLE),
                        None,
                    )
                    .await
                    .map_err(debug)?,
                committed
            );
            let respawned = h
                .root
                .consume_pending_respawn(&authority, &h.node, fence(3)?, Some(death.occurrence))
                .await
                .map_err(debug)?
                .ok_or("missing pending respawn")?;
            assert_eq!(respawned.occurrence, death.occurrence);
            assert_eq!(respawned.respawn_position, death.respawn_position);
            assert_eq!(h.count("game_character_pending_respawns").await?, 0);
            assert_eq!(h.count("game_character_death_receipts").await?, 1);
            assert_eq!(h.root_revision().await?, "3");
            Ok(())
        })
    }

    #[test]
    fn native_trained_death_preserves_build_without_a_current_formula() -> TestResult {
        for stale_formula in [false, true] {
            run("death_build_no_formula", async |h| {
                let trained = train(h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let stale = Table([17; 32], "stale-content");
                let formula = stale_formula.then_some(&stale as &dyn BuildFormula);
                let committed = h
                    .root
                    .settle_runtime_player_death(
                        &CharacterRevisionSequencer::new(),
                        &authority,
                        &h.node,
                        fence(2)?,
                        request(95)?,
                        formula,
                        None,
                    )
                    .await
                    .map_err(debug)?;
                assert!(committed.build_loss.is_none());
                assert_eq!(committed.experience_lost.get(), 1000);
                assert_eq!(
                    h.root
                        .read_character_build_state(&authority, fence(3)?.character_id)
                        .await
                        .map_err(debug)?,
                    trained
                );
                assert_eq!(h.root_revision().await?, "3");
                Ok(())
            })?;
        }
        Ok(())
    }

    async fn join_two<A: Future, B: Future>(a: A, b: B) -> (A::Output, B::Output) {
        let mut a = std::pin::pin!(a);
        let mut b = std::pin::pin!(b);
        let (mut left, mut right) = (None, None);
        poll_fn(move |cx| {
            if left.is_none()
                && let Poll::Ready(value) = a.as_mut().poll(cx)
            {
                left = Some(value);
            }
            if right.is_none()
                && let Poll::Ready(value) = b.as_mut().poll(cx)
            {
                right = Some(value);
            }
            if left.is_some() && right.is_some() {
                match (left.take(), right.take()) {
                    (Some(left), Some(right)) => Poll::Ready((left, right)),
                    _ => Poll::Pending,
                }
            } else {
                Poll::Pending
            }
        })
        .await
    }

    #[test]
    fn concurrent_native_deaths_serialize_build_and_one_receipt_on_the_root() -> TestResult {
        run("death_build_concurrent", async |h| {
            train(h).await?;
            let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
            assert!(other.maintain_ready_once().await?);
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let other_authority = other.open_character_authority(&seal).await.map_err(debug)?;
            let (left, right) = join_two(
                h.root.commit_character_death_with_build(
                    &authority,
                    &h.node,
                    fence(2)?,
                    request(90)?,
                    &TABLE,
                ),
                other.commit_character_death_with_build(
                    &other_authority,
                    &h.node,
                    fence(2)?,
                    request(91)?,
                    &TABLE,
                ),
            )
            .await;
            assert_eq!(usize::from(left.is_ok()) + usize::from(right.is_ok()), 1);
            let loser = if left.is_err() { left } else { right };
            assert!(matches!(
                loser,
                Err(CharacterProgressionError::CharacterRevisionMismatch)
            ));
            assert_eq!(h.root_revision().await?, "3");
            assert_eq!(h.count("game_character_build_receipts").await?, 1);
            assert_eq!(h.count("game_character_death_receipts").await?, 1);
            assert_eq!(h.count("game_character_pending_respawns").await?, 1);
            Ok(())
        })
    }
}
