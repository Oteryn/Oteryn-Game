//! D1107/CONDITIONS0 durable food and fractional HP/mana regeneration state.
//! Authoring proposal: migration 0094 and registration are not installed here.
//! Actor-end saves use one fenced H-1 CharacterRevision and typed immutable receipt.
//! Admission/death callers remain with ADMIT-CAP-1/DEATH-ML-LOSS-1.

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

type VitalsResult<T> = std::result::Result<T, CharacterProgressionError>;
const COMMAND_BINDING_VERSION: u8 = 1;
/// CONDITIONS0 §6.3 storage cap; eating still refuses a total reaching this cap.
pub const MAX_DURABLE_FOOD_MS: u32 = 1_200_000;
/// Largest accepted health regeneration period in the vocation table.
pub const MAX_HEALTH_REGEN_PERIOD_MS: u32 = 12_000;
/// Largest accepted mana regeneration period in the vocation table.
pub const MAX_MANA_REGEN_PERIOD_MS: u32 = 6_000;

/// All three fields form one typed state; none may be restored independently.
/// Food can expire with a fractional pool period still accrued in the real actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DurableVitalsResiduals {
    food_remaining_ms: u32,
    regen_health_ms: u32,
    regen_mana_ms: u32,
}
impl DurableVitalsResiduals {
    pub const fn new(
        food_remaining_ms: u32,
        regen_health_ms: u32,
        regen_mana_ms: u32,
    ) -> VitalsResult<Self> {
        if food_remaining_ms > MAX_DURABLE_FOOD_MS
            || regen_health_ms >= MAX_HEALTH_REGEN_PERIOD_MS
            || regen_mana_ms >= MAX_MANA_REGEN_PERIOD_MS
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        Ok(Self {
            food_remaining_ms,
            regen_health_ms,
            regen_mana_ms,
        })
    }
    #[must_use]
    pub const fn food_remaining_ms(self) -> u32 {
        self.food_remaining_ms
    }
    #[must_use]
    pub const fn regen_health_ms(self) -> u32 {
        self.regen_health_ms
    }
    #[must_use]
    pub const fn regen_mana_ms(self) -> u32 {
        self.regen_mana_ms
    }
    fn columns(self) -> (i32, i32, i32) {
        // Constructor bounds are all below i32::MAX.
        (
            self.food_remaining_ms as i32,
            self.regen_health_ms as i32,
            self.regen_mana_ms as i32,
        )
    }
    fn stored(food: i32, health: i32, mana: i32) -> std::result::Result<Self, DurabilityError> {
        let invalid = |_| DurabilityError::InvalidStoredState;
        Self::new(
            u32::try_from(food).map_err(invalid)?,
            u32::try_from(health).map_err(invalid)?,
            u32::try_from(mana).map_err(invalid)?,
        )
        .map_err(|_| DurabilityError::InvalidStoredState)
    }
}

/// The idempotency identity of one actor-end save (UUIDv7), issued once by the owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VitalsResidualSaveOccurrence([u8; 16]);

impl VitalsResidualSaveOccurrence {
    pub fn from_bytes(bytes: [u8; 16]) -> VitalsResult<Self> {
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
pub struct VitalsResidualSaveRequest {
    pub occurrence: VitalsResidualSaveOccurrence,
    pub state: DurableVitalsResiduals,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommittedVitalsResidualSave {
    pub occurrence: VitalsResidualSaveOccurrence,
    pub character_id: CharacterId,
    pub original_character_revision: CharacterRevision,
    pub committed_character_revision: CharacterRevision,
    pub before: DurableVitalsResiduals,
    pub after: DurableVitalsResiduals,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VitalsResidualSaveOutcome {
    Committed(CommittedVitalsResidualSave),
    AlreadyCommitted(CommittedVitalsResidualSave),
    /// The stored values already equal the actor's: nothing was written and the
    /// CharacterRevision is unchanged. The fence was still proven current.
    Unchanged,
}

impl DurabilityRoot {
    /// Save the actor's remaining food time and HP/mana fractional regeneration progress at actor end. Exact occurrence
    /// replay returns the retained receipt without reacquiring session authority; changed
    /// semantic reuse conflicts. A new occurrence is fenced exactly like
    /// `commit_character_experience` and serializes with every Character writer on the
    /// `character_root` row lock. While a death's respawn is pending the stored values are the
    /// death's zeros, and a save that would change them is refused.
    /// Runtime callers reach it only through a
    /// [`RevisionSlot`](super::character_revision_sequencer::RevisionSlot) (CHAR-REV-SEQ-1).
    pub async fn commit_character_vitals_residual_save(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: VitalsResidualSaveRequest,
    ) -> VitalsResult<VitalsResidualSaveOutcome> {
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
                         'oteryn:character-vitals-residual:' || encode($1, 'hex'), 0))",
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
                        return Ok(Ok(VitalsResidualSaveOutcome::AlreadyCommitted(committed)));
                    }

                    // The XP writer's own fence, so every Character write is fenced identically.
                    let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => root,
                        Err(error) => return Ok(Err(error)),
                    };

                    let state = sqlx::query(
                        "SELECT character_revision::text, profile_revision, ruleset_revision, \
                                content_revision, food_remaining_ms, regen_health_ms, regen_mana_ms \
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
                    let before = DurableVitalsResiduals::stored(
                        state.try_get("food_remaining_ms")?,
                        state.try_get("regen_health_ms")?,
                        state.try_get("regen_mana_ms")?,
                    )?;
                    if before == request.state {
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(VitalsResidualSaveOutcome::Unchanged));
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
                    let (food_remaining_ms, health_ms, mana_ms) = request.state.columns();
                    let state_update = sqlx::query(
                        "UPDATE game_character_progression_state \
                            SET character_revision = $2::text::numeric(20,0), food_remaining_ms = $4, \
                                regen_health_ms = $5, regen_mana_ms = $6 \
                          WHERE character_id = encode($1,'hex')::uuid \
                            AND character_revision = $3::text::numeric(20,0)",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(committed_revision.to_string())
                    .bind(root.revision.to_string())
                    .bind(food_remaining_ms)
                    .bind(health_ms)
                    .bind(mana_ms)
                    .execute(&mut *tx)
                    .await?;
                    if state_update.rows_affected() != 1 {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let (food_before, health_before, mana_before) = before.columns();
                    let receipt = sqlx::query("INSERT INTO game_character_vitals_residual_receipts(vitals_residual_occurrence_id, command_binding, character_id, original_character_revision, committed_character_revision, level_before, level_after, experience_before, experience_after, food_remaining_ms_before, food_remaining_ms_after, regen_health_ms_before, regen_health_ms_after, regen_mana_ms_before, regen_mana_ms_after, profile_revision, ruleset_revision, content_revision, simulation_revision, evidence_revision, declaration_revision, policy_revision, reward_revision, committed_at) SELECT encode($1,'hex')::uuid, $2, s.character_id, $3::text::numeric(20,0), $4::text::numeric(20,0), s.level, s.level, s.total_experience, s.total_experience, $5, $6, $7, $8, $9, $10, s.profile_revision, s.ruleset_revision, s.content_revision, s.simulation_revision, s.evidence_revision, s.declaration_revision, s.policy_revision, s.reward_revision, floor(extract(epoch FROM clock_timestamp())*1000)::bigint FROM game_character_progression_state s WHERE s.character_id = encode($11,'hex')::uuid")
                    .bind(request.occurrence.0.as_slice())
                    .bind(&binding)
                    .bind(root.revision.to_string())
                    .bind(committed_revision.to_string())
                    .bind(food_before).bind(food_remaining_ms)
                    .bind(health_before).bind(health_ms)
                    .bind(mana_before).bind(mana_ms)
                    .bind(fence.character_id.as_bytes().as_slice())
                    .execute(&mut *tx).await?;
                    if receipt.rows_affected() != 1 {
                        return Err(DurabilityError::InvalidStoredState);
                    }

                    let committed = CommittedVitalsResidualSave {
                        occurrence: request.occurrence,
                        character_id: fence.character_id,
                        original_character_revision: fence.expected_character_revision,
                        committed_character_revision: CharacterRevision::new(committed_revision)
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                        before,
                        after: request.state,
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(VitalsResidualSaveOutcome::Committed(committed)))
                })
            })
            .await?
    }

    /// Read a retained save after a lost response. This proves only what committed for the
    /// occurrence and never reacquires gameplay authority.
    pub async fn reconcile_character_vitals_residual_save(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        occurrence: VitalsResidualSaveOccurrence,
    ) -> VitalsResult<Option<CommittedVitalsResidualSave>> {
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
    /// a restart. A Character without a typed progression row has never saved any value and
    /// reads three zeros. A stored value outside the storage bounds fails the load closed with
    /// `Unavailable(InvalidStoredState)`.
    pub async fn read_character_vitals_residual(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character_id: CharacterId,
    ) -> VitalsResult<DurableVitalsResiduals> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let row = sqlx::query(
                        "SELECT food_remaining_ms, regen_health_ms, regen_mana_ms \
                           FROM game_character_progression_state \
                          WHERE character_id = encode($1,'hex')::uuid",
                    )
                    .bind(character_id.as_bytes().as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let state = match row {
                        Some(row) => DurableVitalsResiduals::stored(
                            row.try_get("food_remaining_ms")?,
                            row.try_get("regen_health_ms")?,
                            row.try_get("regen_mana_ms")?,
                        )?,
                        None => DurableVitalsResiduals::default(),
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
    request: &VitalsResidualSaveRequest,
) -> Vec<u8> {
    let mut semantic = vec![COMMAND_BINDING_VERSION];
    semantic.extend_from_slice(&request.occurrence.0);
    semantic.extend_from_slice(fence.character_id.as_bytes());
    semantic.extend_from_slice(&fence.expected_character_revision.get().to_be_bytes());
    semantic.extend_from_slice(&request.state.food_remaining_ms.to_be_bytes());
    semantic.extend_from_slice(&request.state.regen_health_ms.to_be_bytes());
    semantic.extend_from_slice(&request.state.regen_mana_ms.to_be_bytes());
    let digest: [u8; 32] = Sha256::digest(&semantic).into();
    let mut binding = Vec::with_capacity(33);
    binding.push(COMMAND_BINDING_VERSION);
    binding.extend_from_slice(&digest);
    binding
}

async fn load_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    occurrence: VitalsResidualSaveOccurrence,
) -> std::result::Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(sqlx::query(
        "SELECT vitals_residual_occurrence_id::text, command_binding, character_id::text, \
                original_character_revision::text, committed_character_revision::text, \
                food_remaining_ms_before, food_remaining_ms_after, regen_health_ms_before, \
                regen_health_ms_after, regen_mana_ms_before, regen_mana_ms_after \
           FROM game_character_vitals_residual_receipts \
          WHERE vitals_residual_occurrence_id = encode($1,'hex')::uuid",
    )
    .bind(occurrence.0.as_slice())
    .fetch_optional(&mut **tx)
    .await?)
}

fn decode_receipt(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<CommittedVitalsResidualSave, DurabilityError> {
    let invalid = |_| DurabilityError::InvalidStoredState;
    let binding: Vec<u8> = row.try_get("command_binding")?;
    let original = numeric_u64(row, "original_character_revision")?;
    let committed = numeric_u64(row, "committed_character_revision")?;
    if !valid_retained_header(&binding, original, committed) {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(CommittedVitalsResidualSave {
        occurrence: VitalsResidualSaveOccurrence::from_bytes(uuid_text(
            row.try_get("vitals_residual_occurrence_id")?,
        )?)
        .map_err(|_| DurabilityError::InvalidStoredState)?,
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
        before: DurableVitalsResiduals::stored(
            row.try_get("food_remaining_ms_before")?,
            row.try_get("regen_health_ms_before")?,
            row.try_get("regen_mana_ms_before")?,
        )?,
        after: DurableVitalsResiduals::stored(
            row.try_get("food_remaining_ms_after")?,
            row.try_get("regen_health_ms_after")?,
            row.try_get("regen_mana_ms_after")?,
        )?,
    })
}

// One pure arithmetic definition is shared by native owner and existing standalone tests.
#[path = "character_vitals_stamina_math.rs"]
mod stamina_math;
pub use stamina_math::{
    MAX_OFFLINE_SECONDS, MAX_STAMINA_MINUTES, StaminaOutOfRange, regenerate_stamina_minutes,
};

/// Recovery-only consistency check. This is historical evidence, never current actor authority.
/// The main Character integrity owner must also count residual receipts in its global chain.
pub(super) async fn verify_vitals_residual_history(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> std::result::Result<(), DurabilityError> {
    let bad = sqlx::query(
        "WITH transitions AS (
           SELECT character_id, committed_character_revision AS revision,
                  food_remaining_ms_before AS food_before, regen_health_ms_before AS health_before,
                  regen_mana_ms_before AS mana_before, food_remaining_ms_after AS food_after,
                  regen_health_ms_after AS health_after, regen_mana_ms_after AS mana_after
             FROM game_character_vitals_residual_receipts
           UNION ALL SELECT character_id, committed_character_revision, NULL, NULL, NULL, 0, 0, 0
             FROM game_character_death_receipts
         ), ordered AS (
           SELECT *, lag(food_after,1,0) OVER w AS previous_food,
                     lag(health_after,1,0) OVER w AS previous_health,
                     lag(mana_after,1,0) OVER w AS previous_mana
             FROM transitions WINDOW w AS (PARTITION BY character_id ORDER BY revision)
         ) SELECT 1 FROM ordered
             WHERE food_before IS NOT NULL AND (food_before,health_before,mana_before)
                 IS DISTINCT FROM (previous_food,previous_health,previous_mana)
         UNION ALL SELECT 1 FROM game_character_progression_state s
             LEFT JOIN LATERAL (SELECT food_after,health_after,mana_after FROM transitions t
                 WHERE t.character_id=s.character_id ORDER BY revision DESC LIMIT 1) t ON true
             WHERE (s.food_remaining_ms,s.regen_health_ms,s.regen_mana_ms)
                 IS DISTINCT FROM (coalesce(t.food_after,0),coalesce(t.health_after,0),coalesce(t.mana_after,0))
         LIMIT 1",
    ).fetch_optional(&mut **tx).await?;
    if bad.is_some() {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}

fn valid_retained_header(binding: &[u8], original: u64, committed: u64) -> bool {
    binding.len() == 33
        && binding.first() == Some(&COMMAND_BINDING_VERSION)
        && original > 0
        && original.checked_add(1) == Some(committed)
}

#[cfg(test)]
mod residual_tests {
    use super::*;

    #[test]
    fn whole_triple_is_bounded_without_discarding_an_unfed_fraction() {
        assert!(DurableVitalsResiduals::new(MAX_DURABLE_FOOD_MS, 11999, 5999).is_ok());
        assert!(DurableVitalsResiduals::new(0, 5000, 1000).is_ok());
        for (food, hp, mp) in [(1200001, 0, 0), (0, 12000, 0), (0, 0, 6000)] {
            assert!(DurableVitalsResiduals::new(food, hp, mp).is_err());
        }
        for (food, hp, mp) in [(-1, 0, 0), (0, -1, 0), (0, 0, -1)] {
            assert!(matches!(
                DurableVitalsResiduals::stored(food, hp, mp),
                Err(DurabilityError::InvalidStoredState)
            ));
        }
        assert_eq!(DurableVitalsResiduals::default().columns(), (0, 0, 0));
    }

    #[test]
    fn retained_header_refuses_unknown_version_length_or_revision_gap() {
        let mut binding = vec![COMMAND_BINDING_VERSION; 33];
        assert!(valid_retained_header(&binding, 1, 2));
        binding[0] = COMMAND_BINDING_VERSION + 1;
        assert!(!valid_retained_header(&binding, 1, 2));
        binding[0] = COMMAND_BINDING_VERSION;
        assert!(!valid_retained_header(&binding[..32], 1, 2));
        assert!(!valid_retained_header(&binding, 0, 1));
        assert!(!valid_retained_header(&binding, 1, 3));
        assert!(!valid_retained_header(&binding, u64::MAX, 0));
    }
}
