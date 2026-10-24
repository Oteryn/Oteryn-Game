//! Durable monk Harmony and remaining forced Serene time (SPELL-D8 H-1, migration 0026).
//!
//! `docs/architecture/OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md` §8.2:
//! Harmony (0..5) and the remaining forced Serene time (owner decision Q1=b) are GAME-CHAR
//! Character state. A new runtime actor loads them ([`DurabilityRoot::read_character_monk_state`])
//! and passes them to `spell::harmony::MonkState::load`. The Serene flag is runtime-actor-local and
//! never stored (D209).
//!
//! There are two save points. At actor end, before the Character lease is released, the owner
//! writes the actor's values with [`DurabilityRoot::commit_character_monk_state_save`]: one
//! Character transaction under the same current gameplay fence as an XP award (recovery fence,
//! FND-04 session generation, lease and scope, current scope assignment and node incarnation, the
//! root row-locked at the expected CharacterRevision). A changed value advances the
//! CharacterRevision exactly once with a monk state receipt; an unchanged value writes nothing. A
//! stale fence writes nothing. At death, `commit_character_death` empties both values in the
//! DEATH-1 transaction, and the 0026 guard rejects a death transition that does not.

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence, numeric_u64,
    state_matches_root, uuid_text,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::{CharacterId, CharacterRevision};
use crate::foundation::RuntimeScopeRefV1;
use sha2::{Digest, Sha256};
use sqlx::Row;

type Result<T> = std::result::Result<T, CharacterProgressionError>;
const COMMAND_BINDING_VERSION: u8 = 1;

/// Most Harmony a Character stores (§8.2: 0..5); the 0026 CHECK.
pub const MAX_DURABLE_HARMONY: u8 = 5;

/// Longest remaining forced Serene time stored: Focus Serenity forces 7000 ms
/// (`spell::harmony::FOCUS_SERENITY_MICROS`); the 0026 CHECK.
pub const MAX_DURABLE_SERENE_FORCED_MICROS: u64 = 7_000_000;

/// The durable monk values of one Character. Both are 0 for a Character that is not a monk, and
/// for one without a typed progression row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DurableMonkState {
    harmony: u8,
    serene_forced_remaining_micros: u64,
}

impl DurableMonkState {
    /// `serene_forced_remaining_micros` is the forced-until time minus owner time, floored at
    /// zero by the caller. Rejects a Harmony above 5 or a remaining time above 7000 ms.
    pub const fn new(harmony: u8, serene_forced_remaining_micros: u64) -> Result<Self> {
        if harmony > MAX_DURABLE_HARMONY
            || serene_forced_remaining_micros > MAX_DURABLE_SERENE_FORCED_MICROS
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        Ok(Self {
            harmony,
            serene_forced_remaining_micros,
        })
    }

    #[must_use]
    pub const fn harmony(&self) -> u8 {
        self.harmony
    }

    #[must_use]
    pub const fn serene_forced_remaining_micros(&self) -> u64 {
        self.serene_forced_remaining_micros
    }

    /// A stored pair; any value outside the storage bounds is corrupt Character state and fails
    /// closed.
    fn stored(
        harmony: i16,
        serene_forced_remaining_micros: i64,
    ) -> std::result::Result<Self, DurabilityError> {
        let harmony = u8::try_from(harmony).map_err(|_| DurabilityError::InvalidStoredState)?;
        let micros = u64::try_from(serene_forced_remaining_micros)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        Self::new(harmony, micros).map_err(|_| DurabilityError::InvalidStoredState)
    }

    fn columns(self) -> (i16, i64) {
        (
            i16::from(self.harmony),
            // At most 7,000,000, so it always fits.
            i64::try_from(self.serene_forced_remaining_micros).unwrap_or(i64::MAX),
        )
    }
}

/// The idempotency identity of one actor-end save (UUIDv7), issued once by the owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MonkStateSaveOccurrence([u8; 16]);

impl MonkStateSaveOccurrence {
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

/// The actor's values at its end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonkStateSaveRequest {
    pub occurrence: MonkStateSaveOccurrence,
    pub state: DurableMonkState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommittedMonkStateSave {
    pub occurrence: MonkStateSaveOccurrence,
    pub character_id: CharacterId,
    pub original_character_revision: CharacterRevision,
    pub committed_character_revision: CharacterRevision,
    pub before: DurableMonkState,
    pub after: DurableMonkState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonkStateSaveOutcome {
    Committed(CommittedMonkStateSave),
    AlreadyCommitted(CommittedMonkStateSave),
    /// The stored values already equal the actor's: nothing was written and the
    /// CharacterRevision is unchanged. The fence was still proven current.
    Unchanged,
}

impl DurabilityRoot {
    /// Save the actor's Harmony and remaining forced Serene time at actor end. Exact occurrence
    /// replay returns the retained receipt without reacquiring session authority; changed
    /// semantic reuse conflicts. A new occurrence is fenced exactly like
    /// `commit_character_experience` and serializes with every Character writer on the
    /// `character_root` row lock. While a death's respawn is pending the stored values are the
    /// death's zeros, and a save that would change them is refused.
    /// Runtime callers reach it only through a
    /// [`RevisionSlot`](super::character_revision_sequencer::RevisionSlot) (CHAR-REV-SEQ-1).
    pub async fn commit_character_monk_state_save(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: MonkStateSaveRequest,
    ) -> Result<MonkStateSaveOutcome> {
        if fence.character_lease_generation == 0
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let binding = command_binding(&fence, &request);
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
                         'oteryn:character-monk-state:' || encode($1, 'hex'), 0))",
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
                        return Ok(Ok(MonkStateSaveOutcome::AlreadyCommitted(committed)));
                    }

                    // The XP writer's own fence, so every Character write is fenced identically.
                    let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => root,
                        Err(error) => return Ok(Err(error)),
                    };

                    let state = sqlx::query(
                        "SELECT character_revision::text, profile_revision, ruleset_revision, \
                                content_revision, harmony, serene_forced_remaining_micros \
                           FROM game_character_progression_state \
                          WHERE character_id = encode($1,'hex')::uuid FOR UPDATE",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let Some(state) = state else {
                        return Ok(Err(CharacterProgressionError::MissingProgressionState));
                    };
                    if numeric_u64(&state, "character_revision")? != root.revision {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    if !state_matches_root(&state, &root) {
                        return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
                    }
                    let before = DurableMonkState::stored(
                        state.try_get("harmony")?,
                        state.try_get("serene_forced_remaining_micros")?,
                    )?;
                    if before == request.state {
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(MonkStateSaveOutcome::Unchanged));
                    }
                    let pending: bool = sqlx::query_scalar(
                        "SELECT EXISTS (SELECT 1 FROM game_character_pending_respawns \
                          WHERE character_id = encode($1,'hex')::uuid)",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .fetch_one(&mut *tx)
                    .await?;
                    if pending {
                        return Ok(Err(CharacterProgressionError::RespawnPending));
                    }

                    let committed_revision = root
                        .revision
                        .checked_add(1)
                        .ok_or(DurabilityError::InvalidStoredState)?;
                    let root_update = sqlx::query(
                        "UPDATE game_character_roots SET character_revision = $2::text::numeric(20,0) \
                          WHERE character_id = encode($1,'hex')::uuid \
                            AND character_revision = $3::text::numeric(20,0)",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(committed_revision.to_string())
                    .bind(root.revision.to_string())
                    .execute(&mut *tx)
                    .await?;
                    if root_update.rows_affected() != 1 {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let (harmony, micros) = request.state.columns();
                    let state_update = sqlx::query(
                        "UPDATE game_character_progression_state \
                            SET character_revision = $2::text::numeric(20,0), harmony = $4, \
                                serene_forced_remaining_micros = $5 \
                          WHERE character_id = encode($1,'hex')::uuid \
                            AND character_revision = $3::text::numeric(20,0)",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(committed_revision.to_string())
                    .bind(root.revision.to_string())
                    .bind(harmony)
                    .bind(micros)
                    .execute(&mut *tx)
                    .await?;
                    if state_update.rows_affected() != 1 {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let (harmony_before, micros_before) = before.columns();
                    let receipt = sqlx::query(
                        "INSERT INTO game_character_monk_state_receipts(\
                           monk_state_occurrence_id, command_binding, character_id, \
                           original_character_revision, committed_character_revision, \
                           level_before, level_after, experience_before, experience_after, \
                           harmony_before, harmony_after, serene_forced_remaining_micros_before, \
                           serene_forced_remaining_micros_after, profile_revision, \
                           ruleset_revision, content_revision, simulation_revision, \
                           evidence_revision, declaration_revision, policy_revision, \
                           reward_revision, committed_at) \
                         SELECT encode($1,'hex')::uuid, $2, s.character_id, \
                           $3::text::numeric(20,0), $4::text::numeric(20,0), s.level, s.level, \
                           s.total_experience, s.total_experience, $5, $6, $7, $8, \
                           s.profile_revision, s.ruleset_revision, s.content_revision, \
                           s.simulation_revision, s.evidence_revision, s.declaration_revision, \
                           s.policy_revision, s.reward_revision, \
                           floor(extract(epoch FROM statement_timestamp())*1000)::bigint \
                           FROM game_character_progression_state s \
                          WHERE s.character_id = encode($9,'hex')::uuid",
                    )
                    .bind(request.occurrence.0.as_slice())
                    .bind(&binding)
                    .bind(root.revision.to_string())
                    .bind(committed_revision.to_string())
                    .bind(harmony_before)
                    .bind(harmony)
                    .bind(micros_before)
                    .bind(micros)
                    .bind(fence.character_id.as_bytes().as_slice())
                    .execute(&mut *tx)
                    .await?;
                    if receipt.rows_affected() != 1 {
                        return Err(DurabilityError::InvalidStoredState);
                    }

                    let committed = CommittedMonkStateSave {
                        occurrence: request.occurrence,
                        character_id: fence.character_id,
                        original_character_revision: fence.expected_character_revision,
                        committed_character_revision: CharacterRevision::new(committed_revision)
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                        before,
                        after: request.state,
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(MonkStateSaveOutcome::Committed(committed)))
                })
            })
            .await?
    }

    /// Read a retained save after a lost response. This proves only what committed for the
    /// occurrence and never reacquires gameplay authority.
    pub async fn reconcile_character_monk_state_save(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        occurrence: MonkStateSaveOccurrence,
    ) -> Result<Option<CommittedMonkStateSave>> {
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

    /// The durable values a new runtime actor loads: a fresh admission, and the first actor after
    /// a restart. A Character without a typed progression row has never saved either value and
    /// reads 0 and 0. A stored value outside the storage bounds fails the load closed with
    /// `Unavailable(InvalidStoredState)`.
    pub async fn read_character_monk_state(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character_id: CharacterId,
    ) -> Result<DurableMonkState> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let row = sqlx::query(
                        "SELECT harmony, serene_forced_remaining_micros \
                           FROM game_character_progression_state \
                          WHERE character_id = encode($1,'hex')::uuid",
                    )
                    .bind(character_id.as_bytes().as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let state = match row {
                        Some(row) => DurableMonkState::stored(
                            row.try_get("harmony")?,
                            row.try_get("serene_forced_remaining_micros")?,
                        )?,
                        None => DurableMonkState::default(),
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(state))
                })
            })
            .await?
    }
}

/// The semantic identity of a save. Retry-local authority (session, connection, lease and scope
/// generations) is excluded so a replay after a lost response matches.
fn command_binding(
    fence: &CurrentCharacterGameplayFence,
    request: &MonkStateSaveRequest,
) -> Vec<u8> {
    let mut semantic = vec![COMMAND_BINDING_VERSION];
    semantic.extend_from_slice(&request.occurrence.0);
    semantic.extend_from_slice(fence.character_id.as_bytes());
    semantic.extend_from_slice(&fence.expected_character_revision.get().to_be_bytes());
    semantic.push(request.state.harmony);
    semantic.extend_from_slice(&request.state.serene_forced_remaining_micros.to_be_bytes());
    let digest: [u8; 32] = Sha256::digest(&semantic).into();
    let mut binding = Vec::with_capacity(33);
    binding.push(COMMAND_BINDING_VERSION);
    binding.extend_from_slice(&digest);
    binding
}

async fn load_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    occurrence: MonkStateSaveOccurrence,
) -> std::result::Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(sqlx::query(
        "SELECT monk_state_occurrence_id::text, command_binding, character_id::text, \
                original_character_revision::text, committed_character_revision::text, \
                harmony_before, harmony_after, serene_forced_remaining_micros_before, \
                serene_forced_remaining_micros_after \
           FROM game_character_monk_state_receipts \
          WHERE monk_state_occurrence_id = encode($1,'hex')::uuid",
    )
    .bind(occurrence.0.as_slice())
    .fetch_optional(&mut **tx)
    .await?)
}

fn decode_receipt(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<CommittedMonkStateSave, DurabilityError> {
    let invalid = |_| DurabilityError::InvalidStoredState;
    Ok(CommittedMonkStateSave {
        occurrence: MonkStateSaveOccurrence(uuid_text(row.try_get("monk_state_occurrence_id")?)?),
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
        before: DurableMonkState::stored(
            row.try_get("harmony_before")?,
            row.try_get("serene_forced_remaining_micros_before")?,
        )?,
        after: DurableMonkState::stored(
            row.try_get("harmony_after")?,
            row.try_get("serene_forced_remaining_micros_after")?,
        )?,
    })
}

#[cfg(test)]
mod tests {
    use super::{DurableMonkState, MAX_DURABLE_HARMONY, MAX_DURABLE_SERENE_FORCED_MICROS};
    use crate::durability::DurabilityError;

    #[test]
    fn durable_monk_state_admits_only_the_storage_bounds() {
        assert!(
            DurableMonkState::new(MAX_DURABLE_HARMONY, MAX_DURABLE_SERENE_FORCED_MICROS).is_ok()
        );
        assert!(DurableMonkState::new(MAX_DURABLE_HARMONY + 1, 0).is_err());
        assert!(DurableMonkState::new(0, MAX_DURABLE_SERENE_FORCED_MICROS + 1).is_err());
        assert_eq!(
            DurableMonkState::default(),
            DurableMonkState::new(0, 0).unwrap_or_default()
        );
    }

    #[test]
    fn stored_values_outside_the_bounds_fail_closed() {
        assert!(matches!(
            DurableMonkState::stored(6, 0),
            Err(DurabilityError::InvalidStoredState)
        ));
        assert!(matches!(
            DurableMonkState::stored(-1, 0),
            Err(DurabilityError::InvalidStoredState)
        ));
        assert!(matches!(
            DurableMonkState::stored(0, 7_000_001),
            Err(DurabilityError::InvalidStoredState)
        ));
        assert!(matches!(
            DurableMonkState::stored(0, -1),
            Err(DurabilityError::InvalidStoredState)
        ));
        assert_eq!(
            DurableMonkState::stored(5, 7_000_000)
                .map(|state| (state.harmony(), state.serene_forced_remaining_micros()))
                .ok(),
            Some((5, 7_000_000))
        );
    }
}
