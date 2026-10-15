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
//! item; no durable promotion or Premium state exists yet, so the D66
//! promotion reduction never applies.
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
        validate_request(&fence, &request)?;
        let binding = command_binding(&fence, &request)?;
        let policy_digest = policy_digest(&request.policy)?;
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

                    sqlx::query(
                        "SELECT pg_advisory_xact_lock(hashtextextended(\
                         'oteryn:character-death:' || encode($1, 'hex'), 0))",
                    )
                    .bind(request.occurrence.0.as_slice())
                    .execute(&mut *tx)
                    .await?;

                    if let Some(row) = load_receipt(&mut tx, request.occurrence).await? {
                        let stored: Vec<u8> = row.try_get("command_binding")?;
                        if stored != binding {
                            return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
                        }
                        let committed = decode_receipt(&row)?;
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(CharacterDeathOutcome::AlreadyCommitted(committed)));
                    }

                    // The player died in the Channel that owns the session.
                    if fence.runtime_scope
                        != RuntimeScopeRefV1::channel(
                            request.death_cell.world_id,
                            request.death_cell.channel_id,
                        )
                    {
                        return Ok(Err(CharacterProgressionError::AuthorityRejected));
                    }
                    let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => root,
                        Err(error) => return Ok(Err(error)),
                    };
                    let root_revision = root.revision;

                    // The character_root row lock taken by the fence serializes
                    // every Character writer, so these reads need no row lock
                    // (0016 grants the runtime no UPDATE on either relation).
                    let pending: Option<String> = sqlx::query_scalar(
                        "SELECT death_occurrence_id::text FROM game_character_pending_respawns \
                          WHERE character_id = encode($1,'hex')::uuid",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    if pending.is_some() {
                        return Ok(Err(CharacterProgressionError::RespawnPending));
                    }

                    let state = sqlx::query(
                        "SELECT character_revision::text, level, total_experience, \
                                profile_revision, ruleset_revision, content_revision, \
                                simulation_revision, evidence_revision, declaration_revision, \
                                policy_revision, reward_revision \
                           FROM game_character_progression_state \
                          WHERE character_id = encode($1,'hex')::uuid FOR UPDATE",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let Some(state) = state else {
                        return Ok(Err(CharacterProgressionError::MissingProgressionState));
                    };
                    if numeric_u64(&state, "character_revision")? != root_revision
                        || !stored_context_matches(
                            &state,
                            &request.context,
                            &request.policy_revision,
                            &request.reward_revision,
                        )
                        || !state_matches_root(&state, &root)
                    {
                        return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
                    }

                    let held: Vec<String> = sqlx::query_scalar(
                        "SELECT blessing_key COLLATE \"C\" FROM game_character_blessings \
                          WHERE character_id = encode($1,'hex')::uuid ORDER BY 1",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .fetch_all(&mut *tx)
                    .await?;
                    if held != request.held_blessings {
                        return Ok(Err(CharacterProgressionError::HeldBlessingsMismatch));
                    }

                    let level = u32::try_from(state.try_get::<i64, _>("level")?)
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    let snapshot = CurrentProgressionSnapshot {
                        level,
                        total_experience: ExactI64::new(state.try_get("total_experience")?),
                        skill_progression: (),
                        magic_progression: (),
                    };
                    // Every held blessing is a regular one until DEATH-4
                    // admits other kinds; more than D58's seven is rejected
                    // by the calculator.
                    let regular_blessings = u8::try_from(held.len())
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    let operation = ProgressionOperation::ApplyDeathExperienceLoss {
                        death_occurrence: request.occurrence,
                        death_policy_revision: request.policy.death_policy_revision.clone(),
                        declared_difference_revision: request
                            .policy
                            .declared_difference_revision
                            .clone(),
                        regular_blessings,
                        promoted_with_current_premium: false,
                    };
                    let staged = match calculate_progression(
                        &snapshot,
                        &request.context,
                        &request.policy_revision,
                        &operation,
                        &request.policy,
                    ) {
                        Ok(staged) => staged,
                        Err(error) => {
                            return Ok(Err(CharacterProgressionError::Calculation(error)));
                        }
                    };
                    let committed_revision = root_revision
                        .checked_add(1)
                        .ok_or(DurabilityError::InvalidStoredState)?;

                    let root_update = sqlx::query(
                        "UPDATE game_character_roots SET character_revision = $2::text::numeric(20,0) \
                          WHERE character_id = encode($1,'hex')::uuid \
                            AND character_revision = $3::text::numeric(20,0)",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(committed_revision.to_string())
                    .bind(root_revision.to_string())
                    .execute(&mut *tx)
                    .await?;
                    if root_update.rows_affected() != 1 {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let progression_update = sqlx::query(
                        "UPDATE game_character_progression_state \
                            SET character_revision = $2::text::numeric(20,0), level = $3, \
                                total_experience = $4, harmony = 0, \
                                serene_forced_remaining_micros = 0 \
                          WHERE character_id = encode($1,'hex')::uuid \
                            AND character_revision = $5::text::numeric(20,0)",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(committed_revision.to_string())
                    .bind(i64::from(staged.level_after))
                    .bind(staged.experience_after.get())
                    .bind(root_revision.to_string())
                    .execute(&mut *tx)
                    .await?;
                    if progression_update.rows_affected() != 1 {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let committed_at: i64 = sqlx::query_scalar(
                        "SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint",
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    let blessings_after: Vec<String> = Vec::new();
                    sqlx::query(
                        "INSERT INTO game_character_death_receipts(\
                           death_occurrence_id, command_binding, policy_digest, character_id, \
                           original_character_revision, committed_character_revision, \
                           level_before, level_after, experience_before, experience_after, \
                           experience_lost, blessings_before, blessings_after, \
                           amulet_of_loss_item_id, lost_item_ids, death_world_id, \
                           death_channel_id, death_spatial_position, death_map_revision, \
                           respawn_position, death_policy_revision, profile_revision, \
                           ruleset_revision, content_revision, simulation_revision, \
                           evidence_revision, declaration_revision, policy_revision, \
                           reward_revision, committed_at) \
                         VALUES (encode($1,'hex')::uuid, $2, $3, encode($4,'hex')::uuid, \
                           $5::text::numeric(20,0), $6::text::numeric(20,0), $7, $8, $9, $10, \
                           $11, $12, $13, NULL, ARRAY[]::uuid[], encode($14,'hex')::uuid, \
                           encode($15,'hex')::uuid, $16, $17, $18, $19, $20, $21, $22, $23, \
                           $24, $25, $26, $27, $28)",
                    )
                    .bind(request.occurrence.0.as_slice())
                    .bind(&binding)
                    .bind(policy_digest.as_slice())
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(root_revision.to_string())
                    .bind(committed_revision.to_string())
                    .bind(i64::from(staged.level_before))
                    .bind(i64::from(staged.level_after))
                    .bind(staged.experience_before.get())
                    .bind(staged.experience_after.get())
                    .bind(staged.death_experience_lost.get())
                    .bind(&held)
                    .bind(&blessings_after)
                    .bind(request.death_cell.world_id.as_bytes().as_slice())
                    .bind(request.death_cell.channel_id.as_bytes().as_slice())
                    .bind(&request.death_cell.spatial_position)
                    .bind(&request.death_cell.map_revision)
                    .bind(&request.respawn_position)
                    .bind(&request.policy.death_policy_revision)
                    .bind(&request.context.profile)
                    .bind(&request.context.ruleset)
                    .bind(&request.context.content)
                    .bind(&request.context.simulation)
                    .bind(&request.context.evidence)
                    .bind(&request.context.declaration)
                    .bind(&request.policy_revision)
                    .bind(&request.reward_revision)
                    .bind(committed_at)
                    .execute(&mut *tx)
                    .await?;
                    let consumed = sqlx::query(
                        "DELETE FROM game_character_blessings \
                          WHERE character_id = encode($1,'hex')::uuid",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .execute(&mut *tx)
                    .await?;
                    if usize::try_from(consumed.rows_affected()).ok() != Some(held.len()) {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    sqlx::query(
                        "INSERT INTO game_character_pending_respawns \
                         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3)",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(request.occurrence.0.as_slice())
                    .bind(&request.respawn_position)
                    .execute(&mut *tx)
                    .await?;

                    let committed = CommittedCharacterDeath {
                        occurrence: request.occurrence,
                        character_id: fence.character_id,
                        original_character_revision: fence.expected_character_revision,
                        committed_character_revision: CharacterRevision::new(committed_revision)
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                        level_before: staged.level_before,
                        level_after: staged.level_after,
                        experience_before: staged.experience_before,
                        experience_after: staged.experience_after,
                        experience_lost: staged.death_experience_lost,
                        blessings_before: held,
                        blessings_after,
                        death_cell: request.death_cell.clone(),
                        respawn_position: request.respawn_position.clone(),
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(CharacterDeathOutcome::Committed(committed)))
                })
            })
            .await?
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

    /// DEATH-2 (first player death decision §4.3-§4.5): the durable part of
    /// one runtime death, run by the Channel owner off its lane, in the
    /// Character's revision slot.  A retained receipt is replayed at its
    /// original revision; a new death commits at the slot's cursor (a missing
    /// progression row runs the D88 initializer first, as an XP award does, and
    /// an earlier death's unconsumed respawn is consumed first: the actor that
    /// died again was alive and placed, so that respawn has happened).
    /// The death's own pending respawn is left recorded: the caller places the
    /// player at its respawn position and only then consumes it under the
    /// occurrence ([`Self::consume_pending_respawn`]).  A restart before that
    /// consumption leaves the row for the next admission, which places the
    /// new actor at the recorded position (DEATH-0 §3.4).  A retry after any
    /// partial outcome converges: the death replays its first receipt.
    pub async fn settle_player_death<const N: usize>(
        &self,
        sequencer: &CharacterRevisionSequencer,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: CharacterDeathRequest<N>,
    ) -> Result<CommittedCharacterDeath> {
        let occurrence = request.occurrence;
        let mut slot = sequencer.acquire(fence.character_id).await;
        let original = self
            .reconcile_character_death(authority, occurrence)
            .await?
            .map(|death| death.original_character_revision);
        self.settle_player_death_in_slot(&mut slot, authority, node, fence, request, original)
            .await
    }

    /// A13 §4.6: settle an owner-prepared training checkpoint in its own build
    /// transaction before constructing a new death binding. One existing revision
    /// slot covers both transactions; an exact retained death replay skips training.
    ///
    /// This callable seam does not capture/drain the actor's live deltas, refresh
    /// its current session, or implement death build loss/receipt columns. Runtime
    /// adoption requires those owners and the inseparable migration/writer bundle.
    /// `None` must mean the owner has no pending checkpoint, not permission to drop
    /// pending progress. A failed/stale/unknown flush prevents the death; the owner
    /// retains its deltas and retries using the existing training-save rules.
    #[allow(
        clippy::too_many_arguments,
        reason = "existing death inputs plus the owner-prepared checkpoint"
    )]
    pub async fn settle_player_death_with_training<const N: usize>(
        &self,
        sequencer: &CharacterRevisionSequencer,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: CharacterDeathRequest<N>,
        training: Option<DeathTrainingFlush<'_>>,
    ) -> Result<CommittedCharacterDeath> {
        validate_request(&fence, &request)?;
        let mut slot = sequencer.acquire(fence.character_id).await;
        let original = self
            .reconcile_character_death(authority, request.occurrence)
            .await?
            .map(|death| death.original_character_revision);
        if original.is_none()
            && let Some(training) = training
        {
            validate_death_training_input(fence, &training)?;
            settle_death_training_flush(
                fence.character_id,
                training.fence.expected_character_revision,
                training.request,
                async |original| {
                    slot.commit_build(
                        self,
                        authority,
                        node,
                        training.fence,
                        training.request.clone(),
                        training.formula,
                        original,
                    )
                    .await
                },
                async || {
                    self.reconcile_character_build(authority, training.request.occurrence)
                        .await
                },
            )
            .await?;
        }
        // Only the revision cursor is selected here. The supplied identity/fence
        // fields remain intent; the death writer resolves current DB authority.
        let fence = if original.is_none() {
            CurrentCharacterGameplayFence {
                expected_character_revision: slot_cursor(&mut slot, self, authority).await?,
                ..fence
            }
        } else {
            fence
        };
        self.settle_player_death_in_slot(&mut slot, authority, node, fence, request, original)
            .await
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the shared slot and existing death writer inputs"
    )]
    async fn settle_player_death_in_slot<const N: usize>(
        &self,
        slot: &mut RevisionSlot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: CharacterDeathRequest<N>,
        original: Option<CharacterRevision>,
    ) -> Result<CommittedCharacterDeath> {
        let outcome = match slot
            .commit_death(self, authority, node, fence, request.clone(), original)
            .await
        {
            Err(CharacterProgressionError::MissingProgressionState) if original.is_none() => {
                let expected_character_revision = slot_cursor(slot, self, authority).await?;
                self.initialize_character_progression(
                    authority,
                    node,
                    CurrentCharacterGameplayFence {
                        expected_character_revision,
                        ..fence
                    },
                    ProgressionInitializationRequest {
                        context: request.context.clone(),
                        policy_revision: request.policy_revision.clone(),
                        reward_revision: request.reward_revision.clone(),
                        policy: request.policy.clone(),
                    },
                )
                .await?;
                slot.commit_death(self, authority, node, fence, request, None)
                    .await?
            }
            // An earlier death's respawn was never consumed (its admission's consumption
            // failed), yet this actor is alive and placed: that respawn has happened.
            Err(CharacterProgressionError::RespawnPending) if original.is_none() => {
                let expected_character_revision = slot_cursor(slot, self, authority).await?;
                self.consume_pending_respawn(
                    authority,
                    node,
                    CurrentCharacterGameplayFence {
                        expected_character_revision,
                        ..fence
                    },
                    None,
                )
                .await?;
                slot.commit_death(self, authority, node, fence, request, None)
                    .await?
            }
            outcome => outcome?,
        };
        let (CharacterDeathOutcome::Committed(committed)
        | CharacterDeathOutcome::AlreadyCommitted(committed)) = outcome;
        Ok(committed)
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
    let digest: [u8; 32] = Sha256::digest(&semantic).into();
    let mut binding = Vec::with_capacity(33);
    binding.push(DEATH_BINDING_VERSION);
    binding.extend_from_slice(&digest);
    Ok(binding)
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
                respawn_position \
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
