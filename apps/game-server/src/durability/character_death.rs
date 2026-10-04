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

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence, numeric_u64,
    policy_digest, state_matches_root, stored_context_matches, uuid_text, valid_revision,
};
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
