//! Typed, fenced Character experience commits.
//!
//! This component owns durable application and reconciliation only.  It does
//! not prove that Combat legitimately produced an occurrence or select the
//! Reference progression policy.

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation, scope_key};
use super::{DurabilityError, DurabilityRoot};
use crate::domain::progression::{
    CurrentProgressionSnapshot, FiniteProgressionPolicy, ProgressionCalculationError,
    ProgressionOperation, ProgressionRevisionContext, calculate_progression, validate_policy,
};
use crate::domain::{CharacterId, CharacterRevision};
use crate::foundation::{
    ConnectionGeneration, GameSessionId, RuntimeScopeRefV1, ScopeOwnershipGeneration,
};
use oteryn_simulation_determinism::{ExactI64, RoundingMode};
use sha2::{Digest, Sha256};
use sqlx::Row;

type Result<T> = std::result::Result<T, CharacterProgressionError>;
const COMMAND_BINDING_VERSION: u8 = 1;
const POLICY_BINDING_VERSION: u8 = 1;
const MAX_REVISION_BYTES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExperienceRewardOccurrence([u8; 16]);

impl ExperienceRewardOccurrence {
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

/// Current gameplay authority that must still match durable FND-04 state when
/// a previously unseen reward occurrence commits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrentCharacterGameplayFence {
    pub character_id: CharacterId,
    pub game_session_id: GameSessionId,
    pub connection_generation: ConnectionGeneration,
    pub character_lease_generation: u64,
    pub runtime_scope: RuntimeScopeRefV1,
    pub scope_ownership_generation: ScopeOwnershipGeneration,
    pub expected_character_revision: CharacterRevision,
}

/// Complete semantic input to the existing pure progression calculator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperienceAwardRequest<const N: usize> {
    pub occurrence: ExperienceRewardOccurrence,
    pub amount: ExactI64,
    pub context: ProgressionRevisionContext<String>,
    pub policy_revision: String,
    pub reward_revision: String,
    pub policy: FiniteProgressionPolicy<String, N>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterProgressionState {
    pub character_id: CharacterId,
    pub character_revision: CharacterRevision,
    pub level: u32,
    pub total_experience: ExactI64,
    pub context: ProgressionRevisionContext<String>,
    pub policy_revision: String,
    pub reward_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedExperienceAward {
    pub occurrence: ExperienceRewardOccurrence,
    pub character_id: CharacterId,
    pub original_character_revision: CharacterRevision,
    pub committed_character_revision: CharacterRevision,
    pub level_before: u32,
    pub level_after: u32,
    pub experience_before: ExactI64,
    pub experience_after: ExactI64,
    pub experience_awarded: ExactI64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExperienceCommitOutcome {
    Committed(CommittedExperienceAward),
    AlreadyCommitted(CommittedExperienceAward),
}

/// D88 initial typed progression of a bootstrap-only Character.
pub const INITIAL_CHARACTER_LEVEL: u32 = 1;
pub const INITIAL_TOTAL_EXPERIENCE: i64 = 0;

/// Policy binding stored with the initial progression row.  Profile, ruleset
/// and content must equal the Character root; the remaining revisions bind
/// the context the XP writer later requires.  Level and experience are not
/// caller input; the bound policy must map level 1 to 0 experience.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressionInitializationRequest<const N: usize> {
    pub context: ProgressionRevisionContext<String>,
    pub policy_revision: String,
    pub reward_revision: String,
    pub policy: FiniteProgressionPolicy<String, N>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressionInitializationOutcome {
    Initialized(CharacterProgressionState),
    AlreadyInitialized(CharacterProgressionState),
}

#[derive(Debug)]
pub enum CharacterProgressionError {
    InvalidInput,
    AuthorityRejected,
    MissingProgressionState,
    CharacterRevisionMismatch,
    ProgressionContextMismatch,
    ConflictingOccurrence,
    /// A committed death's respawn is still pending (the character is not
    /// playable), so no second death can commit.
    RespawnPending,
    /// The death intent's held blessings are not the durable held set.
    HeldBlessingsMismatch,
    /// A build change does not start from the stored build (or stance, for a prune).
    BuildStateMismatch,
    /// The stance transition does not start from the committed durable slot.
    StanceStateMismatch,
    FamiliarStateMismatch,
    Calculation(ProgressionCalculationError),
    Unavailable(DurabilityError),
}

impl From<DurabilityError> for CharacterProgressionError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl std::fmt::Display for CharacterProgressionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => formatter.write_str("invalid Character progression input"),
            Self::AuthorityRejected => {
                formatter.write_str("Character progression authority rejected")
            }
            Self::MissingProgressionState => {
                formatter.write_str("Character progression state is missing")
            }
            Self::CharacterRevisionMismatch => {
                formatter.write_str("Character revision does not match")
            }
            Self::ProgressionContextMismatch => {
                formatter.write_str("Character progression context does not match")
            }
            Self::ConflictingOccurrence => {
                formatter.write_str("occurrence was reused with different semantics")
            }
            Self::RespawnPending => formatter.write_str("a Character respawn is still pending"),
            Self::HeldBlessingsMismatch => {
                formatter.write_str("death intent blessings are not the held blessings")
            }
            Self::BuildStateMismatch => {
                formatter.write_str("build change does not start from the stored build")
            }
            Self::StanceStateMismatch => {
                formatter.write_str("stance change does not start from the stored stance")
            }
            Self::FamiliarStateMismatch => {
                formatter.write_str("familiar change does not start from the stored state")
            }
            Self::Calculation(error) => {
                write!(formatter, "progression calculation rejected: {error:?}")
            }
            Self::Unavailable(error) => write!(
                formatter,
                "Character progression storage is unavailable: {error:?}"
            ),
        }
    }
}

impl std::error::Error for CharacterProgressionError {}

impl DurabilityRoot {
    /// Commit one positive XP award.  Exact occurrence replay returns its
    /// retained result without reacquiring session authority; changed semantic
    /// reuse conflicts.  A new occurrence is fenced by current recovery, FND-04
    /// session/lease/scope and node-incarnation facts in the same transaction.
    /// Runtime callers reach it only through a
    /// [`RevisionSlot`](super::character_revision_sequencer::RevisionSlot) (CHAR-REV-SEQ-1).
    pub async fn commit_character_experience<const N: usize>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: ExperienceAwardRequest<N>,
    ) -> Result<ExperienceCommitOutcome> {
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
                         'oteryn:character-xp:' || encode($1, 'hex'), 0))",
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
                        return Ok(Ok(ExperienceCommitOutcome::AlreadyCommitted(committed)));
                    }

                    let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => root,
                        Err(error) => return Ok(Err(error)),
                    };
                    let root_revision = root.revision;

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
                        return Ok(Err(
                            CharacterProgressionError::ProgressionContextMismatch,
                        ));
                    }

                    let level_i64: i64 = state.try_get("level")?;
                    let level = u32::try_from(level_i64)
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    let experience: i64 = state.try_get("total_experience")?;
                    let snapshot = CurrentProgressionSnapshot {
                        level,
                        total_experience: ExactI64::new(experience),
                        skill_progression: (),
                        magic_progression: (),
                    };
                    let operation = ProgressionOperation::AwardExperience {
                        source_occurrence: request.occurrence,
                        reward_revision: request.reward_revision.clone(),
                        amount: request.amount,
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
                                total_experience = $4 \
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
                    sqlx::query(
                        "INSERT INTO game_character_xp_receipts(\
                           reward_occurrence_id, command_binding, policy_digest, character_id, \
                           original_character_revision, committed_character_revision, \
                           level_before, level_after, experience_before, experience_after, \
                           experience_awarded, profile_revision, ruleset_revision, \
                           content_revision, simulation_revision, evidence_revision, \
                           declaration_revision, policy_revision, reward_revision, committed_at) \
                         VALUES (encode($1,'hex')::uuid, $2, $3, encode($4,'hex')::uuid, \
                           $5::text::numeric(20,0), $6::text::numeric(20,0), $7, $8, $9, $10, \
                           $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)",
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
                    .bind(staged.experience_awarded.get())
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

                    let committed = CommittedExperienceAward {
                        occurrence: request.occurrence,
                        character_id: fence.character_id,
                        original_character_revision: fence.expected_character_revision,
                        committed_character_revision: CharacterRevision::new(committed_revision)
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                        level_before: staged.level_before,
                        level_after: staged.level_after,
                        experience_before: staged.experience_before,
                        experience_after: staged.experience_after,
                        experience_awarded: staged.experience_awarded,
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(ExperienceCommitOutcome::Committed(committed)))
                })
            })
            .await?
    }

    /// D88: give a bootstrap-only Character (CharacterRevision one without
    /// typed state) its initial progression, level 1 and total experience 0,
    /// exactly once.  Fenced exactly like `commit_character_experience`
    /// (recovery fence, admission relation locks, FND-04 session/lease/scope,
    /// current scope assignment and node incarnation, locked root at the
    /// expected revision) in the same transaction as the write.  The 0009
    /// triggers admit this row at revision one without a receipt, so the
    /// CharacterRevision does not advance.  An existing row is never
    /// overwritten or regressed: the same binding (context, policy and reward
    /// revisions) is an idempotent no-op that returns the stored state, any
    /// other binding fails closed.  The stored state is the constant D88
    /// value, so it depends on no policy content beyond these revisions.
    pub async fn initialize_character_progression<const N: usize>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: ProgressionInitializationRequest<N>,
    ) -> Result<ProgressionInitializationOutcome> {
        validate_initialization(&fence, &request)?;
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
                    let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => root,
                        Err(error) => return Ok(Err(error)),
                    };

                    let existing = sqlx::query(
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
                    if let Some(row) = existing {
                        if numeric_u64(&row, "character_revision")? != root.revision {
                            return Err(DurabilityError::InvalidStoredState);
                        }
                        if !stored_context_matches(
                            &row,
                            &request.context,
                            &request.policy_revision,
                            &request.reward_revision,
                        ) || !state_matches_root(&row, &root)
                        {
                            return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
                        }
                        let state = decode_state(&row, fence.character_id)?;
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(ProgressionInitializationOutcome::AlreadyInitialized(
                            state,
                        )));
                    }
                    // Absence after revision one is corrupt state, never zero.
                    if root.revision != 1 {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    if request.context.profile != root.profile_revision
                        || request.context.ruleset != root.ruleset_revision
                        || request.context.content != root.content_revision
                    {
                        return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
                    }
                    sqlx::query(
                        "INSERT INTO game_character_progression_state(\
                           character_id, character_revision, level, total_experience, \
                           profile_revision, ruleset_revision, content_revision, \
                           simulation_revision, evidence_revision, declaration_revision, \
                           policy_revision, reward_revision) \
                         VALUES (encode($1,'hex')::uuid, 1, $2, $3, $4, $5, $6, $7, $8, \
                           $9, $10, $11)",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(i64::from(INITIAL_CHARACTER_LEVEL))
                    .bind(INITIAL_TOTAL_EXPERIENCE)
                    .bind(&request.context.profile)
                    .bind(&request.context.ruleset)
                    .bind(&request.context.content)
                    .bind(&request.context.simulation)
                    .bind(&request.context.evidence)
                    .bind(&request.context.declaration)
                    .bind(&request.policy_revision)
                    .bind(&request.reward_revision)
                    .execute(&mut *tx)
                    .await?;
                    let state = CharacterProgressionState {
                        character_id: fence.character_id,
                        character_revision: fence.expected_character_revision,
                        level: INITIAL_CHARACTER_LEVEL,
                        total_experience: ExactI64::new(INITIAL_TOTAL_EXPERIENCE),
                        context: request.context.clone(),
                        policy_revision: request.policy_revision.clone(),
                        reward_revision: request.reward_revision.clone(),
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(ProgressionInitializationOutcome::Initialized(state)))
                })
            })
            .await?
    }

    /// Read a retained outcome after a lost response.  This proves only what
    /// committed for the occurrence and never reacquires gameplay authority.
    pub async fn reconcile_character_experience(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        occurrence: ExperienceRewardOccurrence,
    ) -> Result<Option<CommittedExperienceAward>> {
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

    pub async fn read_character_progression(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character_id: CharacterId,
    ) -> Result<Option<CharacterProgressionState>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let row = sqlx::query(
                        "SELECT character_revision::text, level, total_experience, \
                                profile_revision, ruleset_revision, content_revision, \
                                simulation_revision, evidence_revision, declaration_revision, \
                                policy_revision, reward_revision \
                           FROM game_character_progression_state \
                          WHERE character_id = encode($1,'hex')::uuid",
                    )
                    .bind(character_id.as_bytes().as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let state = row
                        .as_ref()
                        .map(|row| decode_state(row, character_id))
                        .transpose()?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(state))
                })
            })
            .await?
    }
}

/// Current Character root facts proven under the gameplay fence.
pub(super) struct FencedCharacterRoot {
    pub(super) revision: u64,
    profile_revision: String,
    ruleset_revision: String,
    content_revision: String,
}

/// The complete current gameplay fence shared by every Character progression
/// write.  The caller has already asserted the recovery fence and taken the
/// admission relation locks in this transaction.  Proves the live FND-04
/// session/lease/scope, the scope assignment held by the current node
/// incarnation, the admission guards, the live root (row-locked) at the
/// expected CharacterRevision and the current Game-owned interpretation.
pub(super) async fn assert_gameplay_fence(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: &CurrentCharacterGameplayFence,
    node: &NodeIncarnationProof,
) -> std::result::Result<
    std::result::Result<FencedCharacterRoot, CharacterProgressionError>,
    DurabilityError,
> {
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Ok(Err(CharacterProgressionError::AuthorityRejected));
    };
    let session = sqlx::query(
        "SELECT account_id::text FROM game_durability_reconnect_sessions \
         WHERE game_session_id = encode($1,'hex')::uuid \
           AND character_id = encode($2,'hex')::uuid \
           AND world_id = encode($3,'hex')::uuid \
           AND runtime_scope_kind = 1 \
           AND runtime_scope_world_id = encode($3,'hex')::uuid \
           AND runtime_scope_channel_id = encode($4,'hex')::uuid \
           AND runtime_scope_instance_id IS NULL \
           AND current_generation = $5::text::numeric(20,0) \
           AND character_lease_generation = $6::text::numeric(20,0) \
           AND scope_ownership_generation = $7::text::numeric(20,0) \
           AND session_state IN (1,2) FOR SHARE",
    )
    .bind(fence.game_session_id.as_bytes().as_slice())
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .bind(fence.connection_generation.get().to_string())
    .bind(fence.character_lease_generation.to_string())
    .bind(fence.scope_ownership_generation.get().to_string())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(session) = session else {
        return Ok(Err(CharacterProgressionError::AuthorityRejected));
    };
    let account_text: String = session.try_get("account_id")?;

    let key = scope_key(world_id, channel_id);
    let fact = node.fact();
    let assignment = sqlx::query(
        "SELECT 1 FROM game_runtime_scope_assignments \
         WHERE scope_key = $1 AND world_id = encode($2,'hex')::uuid \
           AND channel_id = encode($3,'hex')::uuid AND state = 1 \
           AND ownership_generation = $4::text::numeric(20,0) \
           AND holder_node_id = encode($5,'hex')::uuid \
           AND holder_registration_revision = $6::text::numeric(20,0) \
         FOR SHARE",
    )
    .bind(key.as_slice())
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .bind(fence.scope_ownership_generation.get().to_string())
    .bind(fact.node_id().as_bytes().as_slice())
    .bind(fact.registration_revision().to_string())
    .fetch_optional(&mut **tx)
    .await?;
    if assignment.is_none() || !prove_current_incarnation(tx, node).await? {
        return Ok(Err(CharacterProgressionError::AuthorityRejected));
    }

    let guards_ok: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 \
           FROM game_durability_admission_character_guards c \
           JOIN game_durability_admission_account_guards a \
             ON a.account_id = c.account_id \
           JOIN game_durability_admission_runtime_guards g \
             ON g.scope_key = $1 \
          WHERE c.character_id = encode($2,'hex')::uuid \
            AND c.account_id = $3::uuid \
            AND c.world_id = encode($4,'hex')::uuid \
            AND c.eligible \
            AND c.lease_generation = $5::text::numeric(20,0) \
            AND c.holder_game_session_id = encode($6,'hex')::uuid \
            AND a.presence_character_id = c.character_id \
            AND a.holder_game_session_id = c.holder_game_session_id \
            AND g.ready \
            AND g.ownership_generation = $7::text::numeric(20,0))",
    )
    .bind(key.as_slice())
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(&account_text)
    .bind(world_id.as_bytes().as_slice())
    .bind(fence.character_lease_generation.to_string())
    .bind(fence.game_session_id.as_bytes().as_slice())
    .bind(fence.scope_ownership_generation.get().to_string())
    .fetch_one(&mut **tx)
    .await?;
    if !guards_ok {
        return Ok(Err(CharacterProgressionError::AuthorityRejected));
    }

    let root = sqlx::query(
        "SELECT account_id::text, world_id::text, character_revision::text, \
                profile_revision, ruleset_revision, content_revision, \
                starter_template_revision \
           FROM game_character_roots \
          WHERE character_id = encode($1,'hex')::uuid AND lifecycle = 1 \
          FOR UPDATE",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(root) = root else {
        return Ok(Err(CharacterProgressionError::AuthorityRejected));
    };
    if root.try_get::<String, _>("account_id")? != account_text
        || uuid_text(root.try_get("world_id")?)? != *world_id.as_bytes()
    {
        return Ok(Err(CharacterProgressionError::AuthorityRejected));
    }
    let root_revision = numeric_u64(&root, "character_revision")?;
    if root_revision != fence.expected_character_revision.get() {
        return Ok(Err(CharacterProgressionError::CharacterRevisionMismatch));
    }

    // Interpretation rows are immutable (0005 trigger) and a new one is only
    // ever inserted, so a row lock here would serialize nothing; it would
    // also need UPDATE, which the runtime role does not hold (0006).
    let current = sqlx::query(
        "SELECT profile_revision, ruleset_revision, content_revision, \
                starter_template_revision \
           FROM game_character_interpretations \
          ORDER BY interpretation_revision DESC LIMIT 1",
    )
    .fetch_optional(&mut **tx)
    .await?;
    let Some(current) = current else {
        return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
    };
    for column in [
        "profile_revision",
        "ruleset_revision",
        "content_revision",
        "starter_template_revision",
    ] {
        if root.try_get::<String, _>(column)? != current.try_get::<String, _>(column)? {
            return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
        }
    }
    Ok(Ok(FencedCharacterRoot {
        revision: root_revision,
        profile_revision: root.try_get("profile_revision")?,
        ruleset_revision: root.try_get("ruleset_revision")?,
        content_revision: root.try_get("content_revision")?,
    }))
}

fn validate_initialization<const N: usize>(
    fence: &CurrentCharacterGameplayFence,
    request: &ProgressionInitializationRequest<N>,
) -> Result<()> {
    let context = &request.context;
    let policy = &request.policy;
    let starts_at_level_one = policy.thresholds.first().is_some_and(|first| {
        first.level == INITIAL_CHARACTER_LEVEL
            && first.minimum_experience.get() == INITIAL_TOTAL_EXPERIENCE
    });
    if fence.character_lease_generation == 0
        || validate_policy(policy).is_err()
        || !starts_at_level_one
        || policy.terminal_exclusive_experience.get() <= INITIAL_TOTAL_EXPERIENCE
        || *context != policy.context
        || request.policy_revision != policy.policy_revision
        || request.reward_revision != policy.reward_revision
        || ![
            context.profile.as_str(),
            context.ruleset.as_str(),
            context.content.as_str(),
            context.simulation.as_str(),
            context.evidence.as_str(),
            context.declaration.as_str(),
            request.policy_revision.as_str(),
            request.reward_revision.as_str(),
            policy.death_policy_revision.as_str(),
            policy.declared_difference_revision.as_str(),
        ]
        .into_iter()
        .all(valid_revision)
        || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
    {
        return Err(CharacterProgressionError::InvalidInput);
    }
    Ok(())
}

fn validate_request<const N: usize>(
    fence: &CurrentCharacterGameplayFence,
    request: &ExperienceAwardRequest<N>,
) -> Result<()> {
    if fence.character_lease_generation == 0
        || request.amount.get() <= 0
        || !all_revisions(request).all(valid_revision)
        || request.context != request.policy.context
        || request.policy_revision != request.policy.policy_revision
        || request.reward_revision != request.policy.reward_revision
    {
        return Err(CharacterProgressionError::InvalidInput);
    }
    if !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. }) {
        return Err(CharacterProgressionError::InvalidInput);
    }
    Ok(())
}

fn all_revisions<const N: usize>(
    request: &ExperienceAwardRequest<N>,
) -> impl Iterator<Item = &str> {
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

pub(super) fn valid_revision(value: &str) -> bool {
    let mut bytes = value.bytes();
    value.len() <= MAX_REVISION_BYTES
        && bytes
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
}

fn command_binding<const N: usize>(
    fence: &CurrentCharacterGameplayFence,
    request: &ExperienceAwardRequest<N>,
) -> Result<Vec<u8>> {
    let mut semantic = Vec::new();
    semantic.push(COMMAND_BINDING_VERSION);
    semantic.extend_from_slice(&request.occurrence.0);
    semantic.extend_from_slice(fence.character_id.as_bytes());
    semantic.extend_from_slice(&fence.expected_character_revision.get().to_be_bytes());
    semantic.extend_from_slice(&request.amount.get().to_be_bytes());
    semantic.extend_from_slice(&policy_digest(&request.policy)?);
    for revision in all_revisions(request) {
        let length =
            u16::try_from(revision.len()).map_err(|_| CharacterProgressionError::InvalidInput)?;
        semantic.extend_from_slice(&length.to_be_bytes());
        semantic.extend_from_slice(revision.as_bytes());
    }
    let digest: [u8; 32] = Sha256::digest(&semantic).into();
    let mut binding = Vec::with_capacity(33);
    binding.push(COMMAND_BINDING_VERSION);
    binding.extend_from_slice(&digest);
    Ok(binding)
}

pub(super) fn policy_digest<const N: usize>(
    policy: &FiniteProgressionPolicy<String, N>,
) -> Result<[u8; 32]> {
    let mut encoded = Vec::new();
    encoded.push(POLICY_BINDING_VERSION);
    for revision in [
        policy.context.profile.as_str(),
        policy.context.ruleset.as_str(),
        policy.context.content.as_str(),
        policy.context.simulation.as_str(),
        policy.context.evidence.as_str(),
        policy.context.declaration.as_str(),
        policy.policy_revision.as_str(),
        policy.reward_revision.as_str(),
        policy.death_policy_revision.as_str(),
        policy.declared_difference_revision.as_str(),
    ] {
        let length =
            u16::try_from(revision.len()).map_err(|_| CharacterProgressionError::InvalidInput)?;
        encoded.extend_from_slice(&length.to_be_bytes());
        encoded.extend_from_slice(revision.as_bytes());
    }
    encoded.extend_from_slice(
        &u32::try_from(N)
            .map_err(|_| CharacterProgressionError::InvalidInput)?
            .to_be_bytes(),
    );
    for threshold in policy.thresholds {
        encoded.extend_from_slice(&threshold.level.to_be_bytes());
        encoded.extend_from_slice(&threshold.minimum_experience.get().to_be_bytes());
    }
    encoded.extend_from_slice(&policy.terminal_exclusive_experience.get().to_be_bytes());
    encoded.extend_from_slice(&policy.death_loss_numerator.to_be_bytes());
    encoded.extend_from_slice(&policy.death_loss_denominator.to_be_bytes());
    encoded.push(match policy.death_loss_rounding {
        RoundingMode::TowardZero => 1,
        RoundingMode::Floor => 2,
        RoundingMode::Ceiling => 3,
        RoundingMode::NearestTiesToEven => 4,
    });
    Ok(Sha256::digest(encoded).into())
}

async fn load_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    occurrence: ExperienceRewardOccurrence,
) -> std::result::Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(sqlx::query(
        "SELECT reward_occurrence_id::text, command_binding, character_id::text, \
                original_character_revision::text, committed_character_revision::text, \
                level_before, level_after, experience_before, experience_after, \
                experience_awarded \
           FROM game_character_xp_receipts \
          WHERE reward_occurrence_id = encode($1,'hex')::uuid",
    )
    .bind(occurrence.0.as_slice())
    .fetch_optional(&mut **tx)
    .await?)
}

fn decode_receipt(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<CommittedExperienceAward, DurabilityError> {
    let occurrence = ExperienceRewardOccurrence(uuid_text(row.try_get("reward_occurrence_id")?)?);
    let character = CharacterId::from_bytes(uuid_text(row.try_get("character_id")?)?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let original = CharacterRevision::new(numeric_u64(row, "original_character_revision")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let committed = CharacterRevision::new(numeric_u64(row, "committed_character_revision")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let level_before = u32::try_from(row.try_get::<i64, _>("level_before")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let level_after = u32::try_from(row.try_get::<i64, _>("level_after")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(CommittedExperienceAward {
        occurrence,
        character_id: character,
        original_character_revision: original,
        committed_character_revision: committed,
        level_before,
        level_after,
        experience_before: ExactI64::new(row.try_get("experience_before")?),
        experience_after: ExactI64::new(row.try_get("experience_after")?),
        experience_awarded: ExactI64::new(row.try_get("experience_awarded")?),
    })
}

fn decode_state(
    row: &sqlx::postgres::PgRow,
    character_id: CharacterId,
) -> std::result::Result<CharacterProgressionState, DurabilityError> {
    let level = u32::try_from(row.try_get::<i64, _>("level")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(CharacterProgressionState {
        character_id,
        character_revision: CharacterRevision::new(numeric_u64(row, "character_revision")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        level,
        total_experience: ExactI64::new(row.try_get("total_experience")?),
        context: ProgressionRevisionContext {
            profile: row.try_get("profile_revision")?,
            ruleset: row.try_get("ruleset_revision")?,
            content: row.try_get("content_revision")?,
            simulation: row.try_get("simulation_revision")?,
            evidence: row.try_get("evidence_revision")?,
            declaration: row.try_get("declaration_revision")?,
        },
        policy_revision: row.try_get("policy_revision")?,
        reward_revision: row.try_get("reward_revision")?,
    })
}

pub(super) fn stored_context_matches(
    row: &sqlx::postgres::PgRow,
    context: &ProgressionRevisionContext<String>,
    policy_revision: &str,
    reward_revision: &str,
) -> bool {
    [
        ("profile_revision", context.profile.as_str()),
        ("ruleset_revision", context.ruleset.as_str()),
        ("content_revision", context.content.as_str()),
        ("simulation_revision", context.simulation.as_str()),
        ("evidence_revision", context.evidence.as_str()),
        ("declaration_revision", context.declaration.as_str()),
        ("policy_revision", policy_revision),
        ("reward_revision", reward_revision),
    ]
    .into_iter()
    .all(|(column, expected)| row.try_get::<String, _>(column).ok().as_deref() == Some(expected))
}

pub(super) fn state_matches_root(row: &sqlx::postgres::PgRow, root: &FencedCharacterRoot) -> bool {
    row.try_get::<String, _>("profile_revision").ok().as_deref()
        == Some(root.profile_revision.as_str())
        && row.try_get::<String, _>("ruleset_revision").ok().as_deref()
            == Some(root.ruleset_revision.as_str())
        && row.try_get::<String, _>("content_revision").ok().as_deref()
            == Some(root.content_revision.as_str())
}

pub(super) fn numeric_u64(
    row: &sqlx::postgres::PgRow,
    column: &str,
) -> std::result::Result<u64, DurabilityError> {
    row.try_get::<String, _>(column)?
        .parse()
        .map_err(|_| DurabilityError::InvalidStoredState)
}

pub(super) fn uuid_text(value: &str) -> std::result::Result<[u8; 16], DurabilityError> {
    let hex: String = value
        .chars()
        .filter(|character| *character != '-')
        .collect();
    if hex.len() != 32 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut out = [0_u8; 16];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
    }
    Ok(out)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::domain::progression::LevelThreshold;
    use crate::foundation::{ChannelId, WorldId};

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

    fn request() -> ExperienceAwardRequest<2> {
        let context = context();
        ExperienceAwardRequest {
            occurrence: ExperienceRewardOccurrence::from_bytes(id(1)).expect("occurrence"),
            amount: ExactI64::new(5),
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
                        level: 50,
                        minimum_experience: ExactI64::new(1000),
                    },
                    LevelThreshold {
                        level: 51,
                        minimum_experience: ExactI64::new(1100),
                    },
                ],
                terminal_exclusive_experience: ExactI64::new(1200),
                death_loss_numerator: 1,
                death_loss_denominator: 1,
                death_loss_rounding: RoundingMode::Floor,
            },
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
            expected_character_revision: CharacterRevision::new(1).expect("revision"),
        }
    }

    #[test]
    fn semantic_binding_excludes_retry_local_authority_but_includes_policy_bytes() {
        let first_fence = fence();
        let award = request();
        let binding = command_binding(&first_fence, &award).expect("binding");
        assert_eq!(binding.len(), 33);

        let mut reconnected = first_fence;
        reconnected.game_session_id = GameSessionId::decode(&id(9)).expect("session");
        reconnected.connection_generation = ConnectionGeneration::new(7).expect("connection");
        reconnected.character_lease_generation = 8;
        reconnected.scope_ownership_generation = ScopeOwnershipGeneration::new(9).expect("scope");
        assert_eq!(
            command_binding(&reconnected, &award).expect("reconnected binding"),
            binding
        );

        let mut changed_policy = award.clone();
        changed_policy.policy.thresholds[1].minimum_experience = ExactI64::new(1101);
        assert_ne!(
            command_binding(&first_fence, &changed_policy).expect("changed binding"),
            binding
        );
        let mut changed_amount = award;
        changed_amount.amount = ExactI64::new(6);
        assert_ne!(
            command_binding(&first_fence, &changed_amount).expect("changed amount"),
            binding
        );
    }

    #[test]
    fn invalid_occurrence_and_unbound_context_fail_before_database_work() {
        assert!(matches!(
            ExperienceRewardOccurrence::from_bytes([0; 16]),
            Err(CharacterProgressionError::InvalidInput)
        ));
        let mut award = request();
        award.context.content = "content-2".into();
        assert!(matches!(
            validate_request(&fence(), &award),
            Err(CharacterProgressionError::InvalidInput)
        ));
    }

    #[test]
    fn initialization_requires_a_policy_mapping_level_one_to_zero_experience() {
        let award = request();
        let mut initialization = ProgressionInitializationRequest {
            context: award.context,
            policy_revision: award.policy_revision,
            reward_revision: award.reward_revision,
            policy: award.policy,
        };
        // The fixture policy starts at level 50 / 1000 experience.
        assert!(matches!(
            validate_initialization(&fence(), &initialization),
            Err(CharacterProgressionError::InvalidInput)
        ));
        initialization.policy.thresholds = [
            LevelThreshold {
                level: 1,
                minimum_experience: ExactI64::new(0),
            },
            LevelThreshold {
                level: 2,
                minimum_experience: ExactI64::new(100),
            },
        ];
        validate_initialization(&fence(), &initialization).expect("level one policy");
        initialization.policy.thresholds[0].minimum_experience = ExactI64::new(1);
        assert!(matches!(
            validate_initialization(&fence(), &initialization),
            Err(CharacterProgressionError::InvalidInput)
        ));
        // A level-one start is not enough: the whole policy must be one the
        // XP writer can later use (here, non-consecutive levels).
        initialization.policy.thresholds[0].minimum_experience = ExactI64::new(0);
        initialization.policy.thresholds[1].level = 3;
        assert!(matches!(
            validate_initialization(&fence(), &initialization),
            Err(CharacterProgressionError::InvalidInput)
        ));
        initialization.policy.thresholds[1].level = 2;
        initialization.policy.death_loss_denominator = 0;
        assert!(matches!(
            validate_initialization(&fence(), &initialization),
            Err(CharacterProgressionError::InvalidInput)
        ));
    }
}
