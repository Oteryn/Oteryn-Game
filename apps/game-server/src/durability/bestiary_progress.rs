//! CHARM-2: typed, fenced Bestiary kill progress (migration 0019).
//!
//! One credited kill of a Bestiary race is one Character semantic
//! transaction: it advances the global CharacterRevision by one (DUR-02
//! §4.1), writes one immutable kill receipt keyed by the (death, character)
//! reward occurrence and moves the race's `kill_count` up by one. Experience
//! and level are unchanged. A kill at the race's final threshold is
//! saturated: nothing is written and the revision does not advance.
//!
//! The write uses the R7 P03 XP writer's own fence
//! (`character_progression::assert_gameplay_fence`): recovery fence,
//! admission relation locks, the
//! live FND-04 session/lease/scope, the current scope assignment and node
//! incarnation, the locked live root at the expected CharacterRevision and
//! the current Game-owned interpretation, all in the writing transaction.
//! This component does not prove that Combat legitimately produced the
//! occurrence or that the kill was credited; `combat::death_reward` does.

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence, numeric_u64,
    state_matches_root, stored_context_matches, uuid_text, valid_revision,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::bestiary::BestiaryRace;
use crate::domain::progression::ProgressionRevisionContext;
use crate::domain::{CharacterId, CharacterRevision};
use crate::foundation::RuntimeScopeRefV1;
use sha2::{Digest, Sha256};
use sqlx::Row;

type Result<T> = std::result::Result<T, BestiaryProgressError>;
const COMMAND_BINDING_VERSION: u8 = 1;
const RACE_BINDING_VERSION: u8 = 1;

/// The (death, character) reward occurrence a kill is recorded under: the
/// same UUIDv7 the XP award of that death uses, in its own receipt kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BestiaryKillOccurrence([u8; 16]);

impl BestiaryKillOccurrence {
    pub fn from_bytes(bytes: [u8; 16]) -> Result<Self> {
        if bytes[6] >> 4 != 7 || bytes[8] & 0xc0 != 0x80 {
            return Err(BestiaryProgressError::InvalidInput);
        }
        Ok(Self(bytes))
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

/// Complete semantic input of one credited kill. The context, policy and
/// reward revisions are the progression binding the death's XP award uses;
/// they must equal the stored progression state, which the receipt carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BestiaryKillRequest {
    pub occurrence: BestiaryKillOccurrence,
    pub race: BestiaryRace,
    pub context: ProgressionRevisionContext<String>,
    pub policy_revision: String,
    pub reward_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedBestiaryKill {
    pub occurrence: BestiaryKillOccurrence,
    pub character_id: CharacterId,
    pub race_key: String,
    pub race_definition_revision: String,
    pub original_character_revision: CharacterRevision,
    pub committed_character_revision: CharacterRevision,
    pub final_kill_threshold: u32,
    pub kill_count_before: u32,
    pub kill_count_after: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BestiaryKillOutcome {
    Committed(CommittedBestiaryKill),
    AlreadyCommitted(CommittedBestiaryKill),
    /// The race's count is already at (or, after a definition revision
    /// lowered it, above) the final threshold. Nothing was written.
    Saturated {
        race_key: String,
        kill_count: u32,
    },
}

/// The stored counter of one race. Stages are derived from it with the
/// race's [`BestiaryRace`], never stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BestiaryRaceProgress {
    pub character_id: CharacterId,
    pub race_key: String,
    pub kill_count: u32,
    pub committed_character_revision: CharacterRevision,
}

#[derive(Debug)]
pub enum BestiaryProgressError {
    InvalidInput,
    AuthorityRejected,
    MissingProgressionState,
    CharacterRevisionMismatch,
    ProgressionContextMismatch,
    ConflictingOccurrence,
    Unavailable(DurabilityError),
}

impl From<DurabilityError> for BestiaryProgressError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl std::fmt::Display for BestiaryProgressError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => formatter.write_str("invalid Bestiary progress input"),
            Self::AuthorityRejected => formatter.write_str("Bestiary progress authority rejected"),
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
                formatter.write_str("Bestiary kill occurrence was reused with different semantics")
            }
            Self::Unavailable(error) => {
                write!(
                    formatter,
                    "Bestiary progress storage is unavailable: {error:?}"
                )
            }
        }
    }
}

impl std::error::Error for BestiaryProgressError {}

impl DurabilityRoot {
    /// Record one credited kill. Exact occurrence replay returns its retained
    /// result without reacquiring gameplay authority; changed semantic reuse
    /// conflicts. A new occurrence is fenced by current recovery, FND-04
    /// session/lease/scope and node-incarnation facts in the same transaction.
    /// Runtime callers reach it only through a
    /// [`RevisionSlot`](super::character_revision_sequencer::RevisionSlot) (CHAR-REV-SEQ-1).
    pub async fn commit_bestiary_kill(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: BestiaryKillRequest,
    ) -> Result<BestiaryKillOutcome> {
        validate_request(&fence, &request)?;
        let binding = command_binding(&fence, &request)?;
        let race_digest = race_digest(&request.race)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| BestiaryProgressError::AuthorityRejected)?;
        let node = node.clone();

        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;

                    sqlx::query(
                        "SELECT pg_advisory_xact_lock(hashtextextended(\
                         'oteryn:character-bestiary:' || encode($1, 'hex'), 0))",
                    )
                    .bind(request.occurrence.0.as_slice())
                    .execute(&mut *tx)
                    .await?;

                    if let Some(row) = load_receipt(&mut tx, request.occurrence).await? {
                        let stored: Vec<u8> = row.try_get("command_binding")?;
                        if stored != binding {
                            return Ok(Err(BestiaryProgressError::ConflictingOccurrence));
                        }
                        let committed = decode_receipt(&row)?;
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(BestiaryKillOutcome::AlreadyCommitted(committed)));
                    }

                    // The XP writer's own fence, so the two writes are fenced
                    // identically.
                    let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => root,
                        Err(error) => return Ok(Err(fence_error(error))),
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
                        return Ok(Err(BestiaryProgressError::MissingProgressionState));
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
                        return Ok(Err(BestiaryProgressError::ProgressionContextMismatch));
                    }
                    let level: i64 = state.try_get("level")?;
                    let experience: i64 = state.try_get("total_experience")?;

                    let progress = sqlx::query(
                        "SELECT kill_count FROM game_character_bestiary_progress \
                          WHERE character_id = encode($1,'hex')::uuid AND race_key = $2 \
                          FOR UPDATE",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(request.race.key())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let kill_count_before = match &progress {
                        Some(row) => u32::try_from(row.try_get::<i64, _>("kill_count")?)
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                        None => 0,
                    };
                    let Some(kill_count_after) = request.race.next_kill_count(kill_count_before)
                    else {
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(BestiaryKillOutcome::Saturated {
                            race_key: request.race.key().to_owned(),
                            kill_count: kill_count_before,
                        }));
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
                    let state_update = sqlx::query(
                        "UPDATE game_character_progression_state \
                            SET character_revision = $2::text::numeric(20,0) \
                          WHERE character_id = encode($1,'hex')::uuid \
                            AND character_revision = $3::text::numeric(20,0)",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(committed_revision.to_string())
                    .bind(root_revision.to_string())
                    .execute(&mut *tx)
                    .await?;
                    if state_update.rows_affected() != 1 {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let progress_write = if progress.is_some() {
                        sqlx::query(
                            "UPDATE game_character_bestiary_progress \
                                SET kill_count = $3, \
                                    committed_character_revision = $4::text::numeric(20,0), \
                                    last_bestiary_occurrence_id = encode($5,'hex')::uuid \
                              WHERE character_id = encode($1,'hex')::uuid AND race_key = $2 \
                                AND kill_count = $6",
                        )
                        .bind(fence.character_id.as_bytes().as_slice())
                        .bind(request.race.key())
                        .bind(i64::from(kill_count_after))
                        .bind(committed_revision.to_string())
                        .bind(request.occurrence.0.as_slice())
                        .bind(i64::from(kill_count_before))
                        .execute(&mut *tx)
                        .await?
                    } else {
                        sqlx::query(
                            "INSERT INTO game_character_bestiary_progress(\
                               character_id, race_key, kill_count, committed_character_revision, \
                               last_bestiary_occurrence_id) \
                             VALUES (encode($1,'hex')::uuid, $2, $3, $4::text::numeric(20,0), \
                               encode($5,'hex')::uuid)",
                        )
                        .bind(fence.character_id.as_bytes().as_slice())
                        .bind(request.race.key())
                        .bind(i64::from(kill_count_after))
                        .bind(committed_revision.to_string())
                        .bind(request.occurrence.0.as_slice())
                        .execute(&mut *tx)
                        .await?
                    };
                    if progress_write.rows_affected() != 1 {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let committed_at: i64 = sqlx::query_scalar(
                        "SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint",
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    sqlx::query(
                        "INSERT INTO game_character_bestiary_kill_receipts(\
                           bestiary_occurrence_id, command_binding, race_digest, character_id, \
                           original_character_revision, committed_character_revision, \
                           level_before, level_after, experience_before, experience_after, \
                           race_key, race_definition_revision, final_kill_threshold, \
                           kill_count_before, kill_count_after, profile_revision, \
                           ruleset_revision, content_revision, simulation_revision, \
                           evidence_revision, declaration_revision, policy_revision, \
                           reward_revision, committed_at) \
                         VALUES (encode($1,'hex')::uuid, $2, $3, encode($4,'hex')::uuid, \
                           $5::text::numeric(20,0), $6::text::numeric(20,0), $7, $7, $8, $8, \
                           $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22)",
                    )
                    .bind(request.occurrence.0.as_slice())
                    .bind(&binding)
                    .bind(race_digest.as_slice())
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(root_revision.to_string())
                    .bind(committed_revision.to_string())
                    .bind(level)
                    .bind(experience)
                    .bind(request.race.key())
                    .bind(request.race.definition_revision())
                    .bind(i64::from(request.race.final_kill_threshold()))
                    .bind(i64::from(kill_count_before))
                    .bind(i64::from(kill_count_after))
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

                    let committed = CommittedBestiaryKill {
                        occurrence: request.occurrence,
                        character_id: fence.character_id,
                        race_key: request.race.key().to_owned(),
                        race_definition_revision: request.race.definition_revision().to_owned(),
                        original_character_revision: fence.expected_character_revision,
                        committed_character_revision: CharacterRevision::new(committed_revision)
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                        final_kill_threshold: request.race.final_kill_threshold(),
                        kill_count_before,
                        kill_count_after,
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(BestiaryKillOutcome::Committed(committed)))
                })
            })
            .await?
    }

    /// Read a retained kill after a lost response. This proves only what
    /// committed for the occurrence and never reacquires gameplay authority.
    pub async fn reconcile_bestiary_kill(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        occurrence: BestiaryKillOccurrence,
    ) -> Result<Option<CommittedBestiaryKill>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| BestiaryProgressError::AuthorityRejected)?;
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

    /// The stored counter of one race; `None` means zero kills.
    pub async fn read_bestiary_progress(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character_id: CharacterId,
        race_key: &str,
    ) -> Result<Option<BestiaryRaceProgress>> {
        if !valid_revision(race_key) {
            return Err(BestiaryProgressError::InvalidInput);
        }
        let race_key = race_key.to_owned();
        let recovery = authority
            .record_for(self)
            .map_err(|_| BestiaryProgressError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let row = sqlx::query(
                        "SELECT kill_count, committed_character_revision::text \
                           FROM game_character_bestiary_progress \
                          WHERE character_id = encode($1,'hex')::uuid AND race_key = $2",
                    )
                    .bind(character_id.as_bytes().as_slice())
                    .bind(&race_key)
                    .fetch_optional(&mut *tx)
                    .await?;
                    let progress = match row {
                        Some(row) => Some(BestiaryRaceProgress {
                            character_id,
                            race_key: race_key.clone(),
                            kill_count: u32::try_from(row.try_get::<i64, _>("kill_count")?)
                                .map_err(|_| DurabilityError::InvalidStoredState)?,
                            committed_character_revision: CharacterRevision::new(numeric_u64(
                                &row,
                                "committed_character_revision",
                            )?)
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                        }),
                        None => None,
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(progress))
                })
            })
            .await?
    }
}

/// The shared fence refuses with `AuthorityRejected`,
/// `CharacterRevisionMismatch` or `ProgressionContextMismatch`; any other
/// refusal it might add later still fails closed as an authority rejection.
fn fence_error(error: CharacterProgressionError) -> BestiaryProgressError {
    match error {
        CharacterProgressionError::CharacterRevisionMismatch => {
            BestiaryProgressError::CharacterRevisionMismatch
        }
        CharacterProgressionError::ProgressionContextMismatch => {
            BestiaryProgressError::ProgressionContextMismatch
        }
        CharacterProgressionError::Unavailable(error) => BestiaryProgressError::Unavailable(error),
        _ => BestiaryProgressError::AuthorityRejected,
    }
}

fn validate_request(
    fence: &CurrentCharacterGameplayFence,
    request: &BestiaryKillRequest,
) -> Result<()> {
    if fence.character_lease_generation == 0
        || !all_revisions(request).all(valid_revision)
        || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
    {
        return Err(BestiaryProgressError::InvalidInput);
    }
    Ok(())
}

fn all_revisions(request: &BestiaryKillRequest) -> impl Iterator<Item = &str> {
    [
        request.context.profile.as_str(),
        request.context.ruleset.as_str(),
        request.context.content.as_str(),
        request.context.simulation.as_str(),
        request.context.evidence.as_str(),
        request.context.declaration.as_str(),
        request.policy_revision.as_str(),
        request.reward_revision.as_str(),
    ]
    .into_iter()
}

fn push_text(encoded: &mut Vec<u8>, value: &str) -> Result<()> {
    let length = u16::try_from(value.len()).map_err(|_| BestiaryProgressError::InvalidInput)?;
    encoded.extend_from_slice(&length.to_be_bytes());
    encoded.extend_from_slice(value.as_bytes());
    Ok(())
}

/// The race exactly as bound at the kill: key, definition revision and every
/// threshold, so a changed definition revision is a different race binding.
fn race_digest(race: &BestiaryRace) -> Result<[u8; 32]> {
    let mut encoded = vec![RACE_BINDING_VERSION];
    push_text(&mut encoded, race.key())?;
    push_text(&mut encoded, race.definition_revision())?;
    encoded.extend_from_slice(
        &u32::try_from(race.kill_thresholds().len())
            .map_err(|_| BestiaryProgressError::InvalidInput)?
            .to_be_bytes(),
    );
    for threshold in race.kill_thresholds() {
        encoded.extend_from_slice(&threshold.to_be_bytes());
    }
    Ok(Sha256::digest(encoded).into())
}

/// The semantic identity of one kill. Unlike the XP binding it excludes the
/// fence's expected CharacterRevision: the composition derives that revision
/// from the sibling XP descendant's outcome, which can legitimately differ
/// between a first attempt and its replay (an XP award that failed first and
/// committed on retry), and that must not turn a replay into a conflict.
/// Session, connection, lease and scope generations are excluded like XP.
fn command_binding(
    fence: &CurrentCharacterGameplayFence,
    request: &BestiaryKillRequest,
) -> Result<Vec<u8>> {
    let mut semantic = vec![COMMAND_BINDING_VERSION];
    semantic.extend_from_slice(&request.occurrence.0);
    semantic.extend_from_slice(fence.character_id.as_bytes());
    semantic.extend_from_slice(&race_digest(&request.race)?);
    for revision in all_revisions(request) {
        push_text(&mut semantic, revision)?;
    }
    let digest: [u8; 32] = Sha256::digest(&semantic).into();
    let mut binding = Vec::with_capacity(33);
    binding.push(COMMAND_BINDING_VERSION);
    binding.extend_from_slice(&digest);
    Ok(binding)
}

async fn load_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    occurrence: BestiaryKillOccurrence,
) -> std::result::Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(sqlx::query(
        "SELECT bestiary_occurrence_id::text, command_binding, character_id::text, \
                original_character_revision::text, committed_character_revision::text, \
                race_key, race_definition_revision, final_kill_threshold, \
                kill_count_before, kill_count_after \
           FROM game_character_bestiary_kill_receipts \
          WHERE bestiary_occurrence_id = encode($1,'hex')::uuid",
    )
    .bind(occurrence.0.as_slice())
    .fetch_optional(&mut **tx)
    .await?)
}

fn decode_receipt(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<CommittedBestiaryKill, DurabilityError> {
    let count = |column: &str| -> std::result::Result<u32, DurabilityError> {
        u32::try_from(row.try_get::<i64, _>(column)?)
            .map_err(|_| DurabilityError::InvalidStoredState)
    };
    Ok(CommittedBestiaryKill {
        occurrence: BestiaryKillOccurrence(uuid_text(row.try_get("bestiary_occurrence_id")?)?),
        character_id: CharacterId::from_bytes(uuid_text(row.try_get("character_id")?)?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        race_key: row.try_get("race_key")?,
        race_definition_revision: row.try_get("race_definition_revision")?,
        original_character_revision: CharacterRevision::new(numeric_u64(
            row,
            "original_character_revision",
        )?)
        .map_err(|_| DurabilityError::InvalidStoredState)?,
        committed_character_revision: CharacterRevision::new(numeric_u64(
            row,
            "committed_character_revision",
        )?)
        .map_err(|_| DurabilityError::InvalidStoredState)?,
        final_kill_threshold: count("final_kill_threshold")?,
        kill_count_before: count("kill_count_before")?,
        kill_count_after: count("kill_count_after")?,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::foundation::{
        ChannelId, ConnectionGeneration, GameSessionId, ScopeOwnershipGeneration, WorldId,
    };

    fn id(seed: u8) -> [u8; 16] {
        [
            seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
        ]
    }

    fn request() -> BestiaryKillRequest {
        BestiaryKillRequest {
            occurrence: BestiaryKillOccurrence::from_bytes(id(1)).expect("occurrence"),
            race: BestiaryRace::new("oteryn:creature.rat", "definition-r1", vec![5, 50, 500])
                .expect("race"),
            context: ProgressionRevisionContext {
                profile: "profile-1".into(),
                ruleset: "ruleset-1".into(),
                content: "content-1".into(),
                simulation: "simulation-1".into(),
                evidence: "evidence-1".into(),
                declaration: "declaration-1".into(),
            },
            policy_revision: "policy-1".into(),
            reward_revision: "reward-1".into(),
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
    fn binding_excludes_retry_local_authority_and_revision_but_includes_the_race() {
        let kill = request();
        let binding = command_binding(&fence(), &kill).expect("binding");
        assert_eq!(binding.len(), 33);

        let mut retried = fence();
        retried.game_session_id = GameSessionId::decode(&id(9)).expect("session");
        retried.connection_generation = ConnectionGeneration::new(7).expect("connection");
        retried.character_lease_generation = 8;
        retried.scope_ownership_generation = ScopeOwnershipGeneration::new(9).expect("scope");
        retried.expected_character_revision = CharacterRevision::new(4).expect("revision");
        assert_eq!(command_binding(&retried, &kill).expect("retried"), binding);

        let mut other_character = fence();
        other_character.character_id = CharacterId::from_bytes(id(6)).expect("character");
        assert_ne!(
            command_binding(&other_character, &kill).expect("character"),
            binding
        );
        for race in [
            BestiaryRace::new(
                "oteryn:creature.cave_rat",
                "definition-r1",
                vec![5, 50, 500],
            ),
            BestiaryRace::new("oteryn:creature.rat", "definition-r2", vec![5, 50, 500]),
            BestiaryRace::new("oteryn:creature.rat", "definition-r1", vec![5, 50, 501]),
        ] {
            let mut changed = kill.clone();
            changed.race = race.expect("race");
            assert_ne!(command_binding(&fence(), &changed).expect("race"), binding);
        }
        let mut changed_context = kill;
        changed_context.context.content = "content-2".into();
        assert_ne!(
            command_binding(&fence(), &changed_context).expect("context"),
            binding
        );
    }

    #[test]
    fn invalid_occurrence_revisions_and_scope_fail_before_database_work() {
        assert!(matches!(
            BestiaryKillOccurrence::from_bytes([0; 16]),
            Err(BestiaryProgressError::InvalidInput)
        ));
        validate_request(&fence(), &request()).expect("valid");
        let mut bad_revision = request();
        bad_revision.reward_revision = "-reward".into();
        assert!(matches!(
            validate_request(&fence(), &bad_revision),
            Err(BestiaryProgressError::InvalidInput)
        ));
        let mut zero_lease = fence();
        zero_lease.character_lease_generation = 0;
        assert!(matches!(
            validate_request(&zero_lease, &request()),
            Err(BestiaryProgressError::InvalidInput)
        ));
        let mut instance = fence();
        instance.runtime_scope = RuntimeScopeRefV1::Instance {
            world_id: WorldId::decode(&id(4)).expect("world"),
            instance_id: id(7),
        };
        assert!(matches!(
            validate_request(&instance, &request()),
            Err(BestiaryProgressError::InvalidInput)
        ));
    }
}
