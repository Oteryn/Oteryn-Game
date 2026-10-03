//! FAMILIAR-1 local candidate: immutable source-qualified appearance/lifetime snapshots.
//! A changed snapshot is one session-generation fenced Character successor, preserving XP,
//! standard stance, build and Monk fields. History replay/reconcile never reacquire authority.
//! See migration 0033; root composition extends the mixed Character receipt chain before use.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence, numeric_u64,
    state_matches_root, uuid_text, valid_revision,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::{CharacterId, CharacterRevision};
use crate::foundation::RuntimeScopeRefV1;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
type Result<T> = std::result::Result<T, CharacterProgressionError>;
const BINDING_VERSION: u8 = 1;

/// Remaining owner-clock duration survives offline time. u64::MAX is the genuine
/// infinite source condition; no monotonic origin or absolute deadline is persisted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FamiliarCooldownSnapshot {
    pub reference_spell_id: u32,
    pub remaining_micros: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DurableFamiliarState {
    pub selected_look: u32,
    pub granted_looks: Vec<u32>,
    pub saved_expiry_unix: i64,
    pub last_logout_unix: i64,
    pub lifecycle_epoch: u64,
    pub familiar_definition: Option<String>,
    pub familiar_revision: Option<String>,
    pub profile_revision: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cooldowns: Vec<FamiliarCooldownSnapshot>,
}
impl Default for DurableFamiliarState {
    fn default() -> Self {
        Self {
            selected_look: 0,
            granted_looks: Vec::new(),
            saved_expiry_unix: 0,
            last_logout_unix: 0,
            lifecycle_epoch: 0,
            familiar_definition: None,
            familiar_revision: None,
            profile_revision: "none".into(),
            cooldowns: Vec::new(),
        }
    }
}
impl DurableFamiliarState {
    pub fn validate(&self) -> Result<()> {
        let valid_key = |key: &str| {
            !key.is_empty()
                && key.len() <= 256
                && key
                    .bytes()
                    .next()
                    .is_some_and(|b| b.is_ascii_alphanumeric())
                && key
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"/._:-".contains(&b))
        };
        if self.cooldowns.len() > 9
            || self.cooldowns.iter().any(|v| v.reference_spell_id == 0)
            || self
                .cooldowns
                .windows(2)
                .any(|p| p[0].reference_spell_id >= p[1].reference_spell_id)
            || self.saved_expiry_unix < 0
            || self.last_logout_unix < 0
            || self.granted_looks.len() > 256
            || self.granted_looks.first() == Some(&0)
            || self.granted_looks.windows(2).any(|p| p[0] >= p[1])
            || self.familiar_definition.is_some() != self.familiar_revision.is_some()
            || self
                .familiar_definition
                .as_deref()
                .is_some_and(|key| !valid_key(key))
            || self
                .familiar_revision
                .as_deref()
                .is_some_and(|r| !valid_revision(r))
            || !valid_revision(&self.profile_revision)
            || serde_json::to_vec(self)
                .map_err(|_| CharacterProgressionError::InvalidInput)?
                .len()
                > 4096
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FamiliarStateOccurrence([u8; 16]);
impl FamiliarStateOccurrence {
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamiliarStateRequest {
    pub occurrence: FamiliarStateOccurrence,
    pub before: DurableFamiliarState,
    pub after: DurableFamiliarState,
    pub content_revision: String,
    pub policy_revision: String,
    pub policy_digest: [u8; 32],
}
impl FamiliarStateRequest {
    fn validate(&self) -> Result<()> {
        self.before.validate()?;
        self.after.validate()?;
        if self.before == self.after
            || !valid_revision(&self.content_revision)
            || !valid_revision(&self.policy_revision)
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DurableFamiliarProjection {
    state: DurableFamiliarState,
    committed_character_revision: Option<CharacterRevision>,
}
impl DurableFamiliarProjection {
    #[must_use]
    pub fn state(&self) -> &DurableFamiliarState {
        &self.state
    }
    #[must_use]
    pub const fn committed_character_revision(&self) -> Option<CharacterRevision> {
        self.committed_character_revision
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedCharacterFamiliar {
    occurrence: FamiliarStateOccurrence,
    character_id: CharacterId,
    original_character_revision: CharacterRevision,
    committed_character_revision: CharacterRevision,
    before: DurableFamiliarState,
    after: DurableFamiliarState,
    content_revision: String,
    policy_revision: String,
    policy_digest: [u8; 32],
    binding: Vec<u8>,
}
impl CommittedCharacterFamiliar {
    #[must_use]
    pub const fn occurrence(&self) -> FamiliarStateOccurrence {
        self.occurrence
    }
    #[must_use]
    pub const fn character_id(&self) -> CharacterId {
        self.character_id
    }
    #[must_use]
    pub const fn original_character_revision(&self) -> CharacterRevision {
        self.original_character_revision
    }
    #[must_use]
    pub const fn committed_character_revision(&self) -> CharacterRevision {
        self.committed_character_revision
    }
    #[must_use]
    pub fn before(&self) -> &DurableFamiliarState {
        &self.before
    }
    #[must_use]
    pub fn after(&self) -> &DurableFamiliarState {
        &self.after
    }
    #[must_use]
    pub fn matches_request(
        &self,
        fence: &CurrentCharacterGameplayFence,
        request: &FamiliarStateRequest,
    ) -> bool {
        request.validate().is_ok()
            && self.occurrence == request.occurrence
            && self.character_id == fence.character_id
            && self.original_character_revision == fence.expected_character_revision
            && self.before == request.before
            && self.after == request.after
            && self.content_revision == request.content_revision
            && self.policy_revision == request.policy_revision
            && self.policy_digest == request.policy_digest
            && command_binding(fence, request).is_ok_and(|binding| binding == self.binding)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FamiliarStateOutcome {
    Committed(CommittedCharacterFamiliar),
    AlreadyCommitted(CommittedCharacterFamiliar),
}
#[derive(Debug)]
pub(crate) enum PendingFamiliarStateOutcome {
    Prepared(PendingCharacterFamiliar),
    AlreadyCommitted(CommittedCharacterFamiliar),
}
#[derive(Debug)]
pub(crate) struct PendingCharacterFamiliar {
    receipt: CommittedCharacterFamiliar,
    physical_transaction: String,
}
impl PendingCharacterFamiliar {
    pub(crate) fn committed_character_revision(&self) -> CharacterRevision {
        self.receipt.committed_character_revision
    }
    pub(super) async fn verify_staged_row(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> std::result::Result<(), DurabilityError> {
        let row = sqlx::query("SELECT created_xact_id::text AS physical,pg_current_xact_id()::text AS current,command_binding FROM game_character_familiar_receipts WHERE familiar_occurrence_id=encode($1,'hex')::uuid")
            .bind(self.receipt.occurrence.0.as_slice()).fetch_optional(&mut **tx).await?
            .ok_or(DurabilityError::InvalidStoredState)?;
        if row.try_get::<String, _>("physical")? != self.physical_transaction
            || row.try_get::<String, _>("current")? != self.physical_transaction
            || row.try_get::<Vec<u8>, _>("command_binding")? != self.receipt.binding
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let actual = load_receipt(tx, self.receipt.occurrence)
            .await?
            .as_ref()
            .map(decode_receipt)
            .transpose()?
            .ok_or(DurabilityError::InvalidStoredState)?;
        if actual != self.receipt {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(())
    }
    /// Only a deadline-bounded transaction owner calls this after observed successful COMMIT.
    pub(super) fn after_successful_commit(self) -> CommittedCharacterFamiliar {
        self.receipt
    }
}

/// Database-clock sample for source Unix-time familiar storage. It is not a session,
/// scope or mutation authority; those independent fences are still required at commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FamiliarUnixTime(i64);
impl FamiliarUnixTime {
    pub(crate) const fn seconds(self) -> i64 {
        self.0
    }
}
impl DurabilityRoot {
    pub(crate) async fn read_familiar_unix_time(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
    ) -> Result<FamiliarUnixTime> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let seconds: i64 = sqlx::query_scalar(
                        "SELECT floor(extract(epoch FROM clock_timestamp()))::bigint",
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    if seconds < 0 {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(FamiliarUnixTime(seconds)))
                })
            })
            .await?
    }
    pub async fn commit_character_familiar_state(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: FamiliarStateRequest,
    ) -> Result<FamiliarStateOutcome> {
        self.commit_character_familiar_state_inner(authority, node, fence, request, true)
            .await
    }

    pub(crate) async fn reconcile_familiar_owner_state(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: FamiliarStateRequest,
    ) -> Result<FamiliarStateOutcome> {
        self.commit_character_familiar_state_inner(authority, node, fence, request, false)
            .await
    }

    async fn commit_character_familiar_state_inner(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: FamiliarStateRequest,
        allow_new_mutation: bool,
    ) -> Result<FamiliarStateOutcome> {
        request.validate()?;
        let binding = command_binding(&fence, &request)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let outcome = apply_familiar_transaction(
                        &mut tx,
                        &recovery,
                        &node,
                        fence,
                        request,
                        binding,
                        allow_new_mutation,
                    )
                    .await?;
                    let outcome = match outcome {
                        Ok(outcome) => outcome,
                        Err(error) => return Ok(Err(error)),
                    };
                    match outcome {
                        PendingFamiliarStateOutcome::Prepared(pending) => {
                            pending.verify_staged_row(&mut tx).await?;
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(FamiliarStateOutcome::Committed(
                                pending.after_successful_commit(),
                            )))
                        }
                        PendingFamiliarStateOutcome::AlreadyCommitted(receipt) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(FamiliarStateOutcome::AlreadyCommitted(receipt)))
                        }
                    }
                })
            })
            .await?
    }

    /// Stage within the common cast transaction. It never commits or upgrades the pending token.
    pub(crate) async fn prepare_character_familiar_in_transaction(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: FamiliarStateRequest,
    ) -> Result<PendingFamiliarStateOutcome> {
        request.validate()?;
        let binding = command_binding(&fence, &request)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        apply_familiar_transaction(tx, &recovery, node, fence, request, binding, true).await?
    }

    pub async fn reconcile_character_familiar_state(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        occurrence: FamiliarStateOccurrence,
    ) -> Result<Option<CommittedCharacterFamiliar>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let receipt = load_receipt(&mut tx, occurrence)
                        .await?
                        .as_ref()
                        .map(decode_receipt)
                        .transpose()?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(receipt))
                })
            })
            .await?
    }

    pub async fn read_character_familiar_state(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character_id: CharacterId,
    ) -> Result<DurableFamiliarProjection> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?.run(move |holder,deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            let row = sqlx::query("SELECT state::text,committed_character_revision::text FROM game_character_familiar_state WHERE character_id=encode($1,'hex')::uuid")
                .bind(character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let projection = row.as_ref().map(decode_projection).transpose()?.unwrap_or_default();
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Ok(projection))
        })).await?
    }
}

fn encode_state(state: &DurableFamiliarState) -> std::result::Result<String, DurabilityError> {
    serde_json::to_string(state).map_err(|_| DurabilityError::InvalidStoredState)
}
fn decode_state(value: String) -> std::result::Result<DurableFamiliarState, DurabilityError> {
    if value.len() > 4096 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let state: DurableFamiliarState =
        serde_json::from_str(&value).map_err(|_| DurabilityError::InvalidStoredState)?;
    state
        .validate()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(state)
}
fn command_binding(
    fence: &CurrentCharacterGameplayFence,
    request: &FamiliarStateRequest,
) -> Result<Vec<u8>> {
    binding_from_identity(
        fence.character_id,
        fence.expected_character_revision,
        request,
    )
}
fn binding_from_identity(
    character_id: CharacterId,
    expected_revision: CharacterRevision,
    request: &FamiliarStateRequest,
) -> Result<Vec<u8>> {
    let mut semantic = vec![BINDING_VERSION];
    semantic.extend_from_slice(&request.occurrence.0);
    semantic.extend_from_slice(character_id.as_bytes());
    semantic.extend_from_slice(&expected_revision.get().to_be_bytes());
    for value in [
        serde_json::to_string(&request.before)
            .map_err(|_| CharacterProgressionError::InvalidInput)?,
        serde_json::to_string(&request.after)
            .map_err(|_| CharacterProgressionError::InvalidInput)?,
        request.content_revision.clone(),
        request.policy_revision.clone(),
    ] {
        semantic.extend_from_slice(&(value.len() as u64).to_be_bytes());
        semantic.extend_from_slice(value.as_bytes());
    }
    semantic.extend_from_slice(&request.policy_digest);
    let mut binding = vec![BINDING_VERSION];
    binding.extend_from_slice(&Sha256::digest(&semantic));
    Ok(binding)
}
async fn apply_familiar_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    request: FamiliarStateRequest,
    binding: Vec<u8>,
    allow_new_mutation: bool,
) -> std::result::Result<Result<PendingFamiliarStateOutcome>, DurabilityError> {
    if fence.character_lease_generation == 0
        || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
    {
        return Ok(Err(CharacterProgressionError::InvalidInput));
    }
    assert_recovery_fence(tx, recovery).await?;
    lock_admission_relations(tx).await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:character-familiar:' || encode($1,'hex'),0))")
                .bind(request.occurrence.0.as_slice()).execute(&mut **tx).await?;
    // Replay precedes current-session checks, but still requires the current recovery seal.
    if let Some(row) = load_receipt(tx, request.occurrence).await? {
        let stored: Vec<u8> = row.try_get("command_binding")?;
        if stored != binding {
            return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
        }
        let receipt = decode_receipt(&row)?;
        return Ok(Ok(PendingFamiliarStateOutcome::AlreadyCommitted(receipt)));
    }
    if !allow_new_mutation {
        return Ok(Err(CharacterProgressionError::AuthorityRejected));
    }
    let root = match assert_gameplay_fence(tx, &fence, &node).await? {
        Ok(root) => root,
        Err(error) => return Ok(Err(error)),
    };
    let state = sqlx::query("SELECT character_revision::text, profile_revision, ruleset_revision, content_revision, policy_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(state) = state else {
        return Ok(Err(CharacterProgressionError::MissingProgressionState));
    };
    if numeric_u64(&state, "character_revision")? != root.revision {
        return Err(DurabilityError::InvalidStoredState);
    }
    if !state_matches_root(&state, &root)
        || state.try_get::<String, _>("content_revision")? != request.content_revision
        || state.try_get::<String, _>("policy_revision")? != request.policy_revision
    {
        return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
    }
    let projection = sqlx::query("SELECT state::text, committed_character_revision::text FROM game_character_familiar_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let before = projection
        .as_ref()
        .map(decode_projection)
        .transpose()?
        .unwrap_or_default();
    if before
        .committed_character_revision
        .is_some_and(|revision| revision.get() > root.revision)
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    if before.state != request.before {
        return Ok(Err(CharacterProgressionError::FamiliarStateMismatch));
    }
    let pending: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_pending_respawns WHERE character_id=encode($1,'hex')::uuid)")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    if pending {
        return Ok(Err(CharacterProgressionError::RespawnPending));
    }
    let next = root
        .revision
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?;
    for query in [
        "UPDATE game_character_roots SET character_revision=$2::text::numeric(20,0) WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric(20,0)",
        "UPDATE game_character_progression_state SET character_revision=$2::text::numeric(20,0) WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric(20,0)",
    ] {
        if sqlx::query(query)
            .bind(fence.character_id.as_bytes().as_slice())
            .bind(next.to_string())
            .bind(root.revision.to_string())
            .execute(&mut **tx)
            .await?
            .rows_affected()
            != 1
        {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    let inserted = sqlx::query("INSERT INTO game_character_familiar_receipts(familiar_occurrence_id,command_binding,policy_digest,character_id,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,state_before,state_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,committed_at) SELECT encode($1,'hex')::uuid,$2,$3,s.character_id,$4::text::numeric(20,0),$5::text::numeric(20,0),s.level,s.level,s.total_experience,s.total_experience,$6::text::jsonb,$7::text::jsonb,s.profile_revision,s.ruleset_revision,s.content_revision,s.simulation_revision,s.evidence_revision,s.declaration_revision,s.policy_revision,s.reward_revision,floor(extract(epoch FROM statement_timestamp())*1000)::bigint FROM game_character_progression_state s WHERE s.character_id=encode($8,'hex')::uuid")
                .bind(request.occurrence.0.as_slice()).bind(&binding).bind(request.policy_digest.as_slice())
                .bind(root.revision.to_string()).bind(next.to_string()).bind(encode_state(&request.before)?).bind(encode_state(&request.after)?)
                .bind(fence.character_id.as_bytes().as_slice()).execute(&mut **tx).await?;
    if inserted.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let updated = sqlx::query("INSERT INTO game_character_familiar_state(character_id,state,committed_character_revision,last_familiar_occurrence_id) VALUES(encode($1,'hex')::uuid,$2::text::jsonb,$3::text::numeric(20,0),encode($4,'hex')::uuid) ON CONFLICT(character_id) DO UPDATE SET state=EXCLUDED.state,committed_character_revision=EXCLUDED.committed_character_revision,last_familiar_occurrence_id=EXCLUDED.last_familiar_occurrence_id")
                .bind(fence.character_id.as_bytes().as_slice()).bind(encode_state(&request.after)?).bind(next.to_string()).bind(request.occurrence.0.as_slice()).execute(&mut **tx).await?;
    if updated.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let receipt = CommittedCharacterFamiliar {
        occurrence: request.occurrence,
        character_id: fence.character_id,
        original_character_revision: fence.expected_character_revision,
        committed_character_revision: CharacterRevision::new(next)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        before: request.before,
        after: request.after,
        content_revision: request.content_revision,
        policy_revision: request.policy_revision,
        policy_digest: request.policy_digest,
        binding,
    };

    let physical_transaction: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
        .fetch_one(&mut **tx)
        .await?;
    Ok(Ok(PendingFamiliarStateOutcome::Prepared(
        PendingCharacterFamiliar {
            receipt,
            physical_transaction,
        },
    )))
}

async fn load_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    occurrence: FamiliarStateOccurrence,
) -> std::result::Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(sqlx::query("SELECT familiar_occurrence_id::text,command_binding,policy_digest,character_id::text,original_character_revision::text,committed_character_revision::text,state_before::text,state_after::text,content_revision,policy_revision FROM game_character_familiar_receipts WHERE familiar_occurrence_id=encode($1,'hex')::uuid").bind(occurrence.0.as_slice()).fetch_optional(&mut **tx).await?)
}
fn decode_projection(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<DurableFamiliarProjection, DurabilityError> {
    let revision = numeric_u64(row, "committed_character_revision")?;
    if revision < 2 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(DurableFamiliarProjection {
        state: decode_state(row.try_get("state")?)?,
        committed_character_revision: Some(
            CharacterRevision::new(revision).map_err(|_| DurabilityError::InvalidStoredState)?,
        ),
    })
}
fn decode_receipt(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<CommittedCharacterFamiliar, DurabilityError> {
    let original = CharacterRevision::new(numeric_u64(row, "original_character_revision")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let committed = CharacterRevision::new(numeric_u64(row, "committed_character_revision")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let before = decode_state(row.try_get("state_before")?)?;
    let after = decode_state(row.try_get("state_after")?)?;
    let binding: Vec<u8> = row.try_get("command_binding")?;
    let digest: Vec<u8> = row.try_get("policy_digest")?;
    let content: String = row.try_get("content_revision")?;
    let policy: String = row.try_get("policy_revision")?;
    if original.get().checked_add(1) != Some(committed.get())
        || before == after
        || binding.len() != 33
        || binding[0] != BINDING_VERSION
        || !valid_revision(&content)
        || !valid_revision(&policy)
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(CommittedCharacterFamiliar {
        occurrence: FamiliarStateOccurrence::from_bytes(uuid_text(
            row.try_get("familiar_occurrence_id")?,
        )?)
        .map_err(|_| DurabilityError::InvalidStoredState)?,
        character_id: CharacterId::from_bytes(uuid_text(row.try_get("character_id")?)?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        original_character_revision: original,
        committed_character_revision: committed,
        before,
        after,
        content_revision: content,
        policy_revision: policy,
        policy_digest: digest
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        binding,
    })
}
/// Admission/recovery integrity, independent of database trigger execution. Checks typed
/// snapshots, full historical bindings, cross-receipt state continuity, projection identity,
/// XP/level invariants and all retained context fields against the existing Character state.
/// This is historical integrity only; it issues no session or gameplay mutation authority.
pub(crate) async fn verify_familiar_chain(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> std::result::Result<(), DurabilityError> {
    let seed = encode_state(&DurableFamiliarState::default())?;
    let corrupt:bool=sqlx::query_scalar(r#"
        WITH ordered AS (
            SELECT f.*,lag(f.state_after,1,$1::text::jsonb) OVER
                (PARTITION BY character_id ORDER BY committed_character_revision) AS previous,
                row_number() OVER (PARTITION BY character_id ORDER BY committed_character_revision DESC) AS latest
            FROM game_character_familiar_receipts f
        ) SELECT EXISTS (
            SELECT 1 FROM ordered f
            LEFT JOIN game_character_roots r ON r.character_id=f.character_id
            LEFT JOIN game_character_progression_state p ON p.character_id=f.character_id
            WHERE r.character_id IS NULL OR p.character_id IS NULL
               OR f.original_character_revision<1 OR f.committed_character_revision<>f.original_character_revision+1
               OR f.committed_character_revision>r.character_revision
               OR f.level_before<1 OR f.level_before>4294967295 OR f.level_before<>f.level_after
               OR f.experience_before<0 OR f.experience_before<>f.experience_after
               OR f.state_before<>f.previous
               OR (f.profile_revision,f.ruleset_revision,f.content_revision,f.simulation_revision,
                   f.evidence_revision,f.declaration_revision,f.policy_revision,f.reward_revision)
                <> (p.profile_revision,p.ruleset_revision,p.content_revision,p.simulation_revision,
                    p.evidence_revision,p.declaration_revision,p.policy_revision,p.reward_revision)
               OR (f.latest=1 AND NOT EXISTS (
                   SELECT 1 FROM game_character_familiar_state s
                    WHERE s.character_id=f.character_id AND s.state=f.state_after
                      AND s.committed_character_revision=f.committed_character_revision
                      AND s.last_familiar_occurrence_id=f.familiar_occurrence_id))
            UNION ALL SELECT 1 FROM game_character_familiar_state s
             WHERE NOT EXISTS(SELECT 1 FROM ordered f WHERE f.character_id=s.character_id)
        )
    "#).bind(seed).fetch_one(&mut **tx).await?;
    if corrupt {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut after = "00000000-0000-0000-0000-000000000000".to_owned();
    loop {
        let rows=sqlx::query("SELECT familiar_occurrence_id::text,command_binding,policy_digest,character_id::text,original_character_revision::text,committed_character_revision::text,state_before::text,state_after::text,content_revision,policy_revision FROM game_character_familiar_receipts WHERE familiar_occurrence_id>$1::uuid ORDER BY familiar_occurrence_id LIMIT 256")
            .bind(&after).fetch_all(&mut **tx).await?;
        let Some(last) = rows.last() else {
            break;
        };
        after = last.try_get("familiar_occurrence_id")?;
        for row in rows {
            let retained = decode_receipt(&row)?;
            let request = FamiliarStateRequest {
                occurrence: retained.occurrence,
                before: retained.before,
                after: retained.after,
                content_revision: retained.content_revision,
                policy_revision: retained.policy_revision,
                policy_digest: retained.policy_digest,
            };
            request
                .validate()
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            let canonical = binding_from_identity(
                retained.character_id,
                retained.original_character_revision,
                &request,
            )
            .map_err(|_| DurabilityError::InvalidStoredState)?;
            if canonical != retained.binding {
                return Err(DurabilityError::InvalidStoredState);
            }
        }
    }
    let mut after = "00000000-0000-0000-0000-000000000000".to_owned();
    loop {
        let rows=sqlx::query("SELECT character_id::text,state::text,committed_character_revision::text FROM game_character_familiar_state WHERE character_id>$1::uuid ORDER BY character_id LIMIT 256").bind(&after).fetch_all(&mut **tx).await?;
        let Some(last) = rows.last() else {
            break;
        };
        after = last.try_get("character_id")?;
        for row in rows {
            decode_projection(&row)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_binding_encoding_survives_optional_offline_cooldown() {
        let legacy = r#"{"selected_look":0,"granted_looks":[],"saved_expiry_unix":0,"last_logout_unix":0,"lifecycle_epoch":0,"familiar_definition":null,"familiar_revision":null,"profile_revision":"none"}"#;
        let state: DurableFamiliarState = serde_json::from_str(legacy).unwrap();
        assert_eq!(serde_json::to_string(&state).unwrap(), legacy);
        let mut with_cooldown = state;
        with_cooldown.cooldowns = vec![FamiliarCooldownSnapshot {
            reference_spell_id: 197,
            remaining_micros: u64::MAX,
        }];
        assert!(with_cooldown.validate().is_ok());
        let encoded = serde_json::to_string(&with_cooldown).unwrap();
        assert_eq!(
            serde_json::from_str::<DurableFamiliarState>(&encoded).unwrap(),
            with_cooldown
        );
        with_cooldown.cooldowns[0].reference_spell_id = 0;
        assert!(with_cooldown.validate().is_err());
        assert!(
            serde_json::from_str::<FamiliarCooldownSnapshot>(
                r#"{"reference_spell_id":197,"remaining_micros":5,"unused":1}"#
            )
            .is_err()
        );
    }
    #[test]
    fn familiar_snapshot_rejects_unknown_or_unsorted_fields_and_preserves_unix_clocks() {
        let mut state = DurableFamiliarState::default();
        assert!(state.validate().is_ok());
        state.selected_look = 991;
        state.granted_looks = vec![991, 993];
        state.saved_expiry_unix = 1_800_000_000;
        state.last_logout_unix = 1_799_999_990;
        assert!(state.validate().is_ok());
        state.granted_looks = vec![993, 991];
        assert!(state.validate().is_err());
        state.granted_looks = vec![991, 991];
        assert!(state.validate().is_err());
        state.granted_looks = vec![0];
        assert!(state.validate().is_err());
        state.granted_looks = vec![991];
        state.familiar_definition = Some("creature:knight/familiar".into());
        assert!(state.validate().is_err());
        state.familiar_revision = Some("r20".into());
        assert!(state.validate().is_ok());
        state.saved_expiry_unix = -1;
        assert!(state.validate().is_err());
        assert!(decode_state("{\"extra\":1}".into()).is_err());
    }
}

/// Both branches contain genuine observed database evidence. A reconciled branch is history;
/// the transport must separately prove its exact retained current owner/predecessor to install.
pub(crate) enum FamiliarSpellCommit {
    Committed {
        common: super::spell_owner_commit::CommittedSpellOwnerTransaction,
        training: Option<super::character_build::BuildCommitOutcome>,
    },
    Reconciled {
        familiar: CommittedCharacterFamiliar,
        cost: super::spell_items_abi::CommittedSpellItems,
        training: Option<super::character_build::BuildCommitOutcome>,
    },
}
impl FamiliarSpellCommit {
    pub(crate) fn familiar(&self) -> Option<&CommittedCharacterFamiliar> {
        match self {
            Self::Committed { common, .. } => common.familiar(),
            Self::Reconciled { familiar, .. } => Some(familiar),
        }
    }
    pub(crate) fn cost(&self) -> Option<&super::spell_items_abi::CommittedSpellItems> {
        match self {
            Self::Committed { common, .. } => common.items(),
            Self::Reconciled { cost, .. } => Some(cost),
        }
    }
    pub(crate) fn training(&self) -> Option<&super::character_build::CommittedBuildChange> {
        let outcome = match self {
            Self::Committed { training, .. } | Self::Reconciled { training, .. } => {
                training.as_ref()?
            }
        };
        match outcome {
            super::character_build::BuildCommitOutcome::Committed(value)
            | super::character_build::BuildCommitOutcome::AlreadyCommitted(value) => Some(value),
        }
    }
}
fn item_failure(error: super::spell_item_transaction::SpellItemError) -> DurabilityError {
    match error {
        super::spell_item_transaction::SpellItemError::Durability(value) => value,
        super::spell_item_transaction::SpellItemError::Database(value) => value.into(),
        super::spell_item_transaction::SpellItemError::Training(error) => match error {
            CharacterProgressionError::Unavailable(value) => value,
            _ => DurabilityError::InvalidStoredState,
        },
        super::spell_item_transaction::SpellItemError::Rejected(_) => {
            DurabilityError::InvalidStoredState
        }
    }
}
impl DurabilityRoot {
    /// One real source cast transaction: familiar state, complete caster costs and an actual
    /// D151 ML advance/checkpoint when requested. No row is written merely for live accumulation.
    /// Physical owners remain outside this SQL-only callback, locked in their established order.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn commit_familiar_spell<
        F: super::character_build::BuildFormula + Clone + Send + Sync + 'static,
    >(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: FamiliarStateRequest,
        cost: super::spell_items_abi::SpellItemTransactionRequest,
        training: Option<(super::character_build::BuildChangeRequest, F)>,
    ) -> Result<FamiliarSpellCommit> {
        self.commit_familiar_spell_inner(authority, node, fence, request, cost, training, true)
            .await
    }

    /// A missing/expired source eligibility observation can only veto a new mutation.
    /// This path still proves all current DB fences and full historical cast joins; it
    /// cannot insert a familiar/cost/training successor when the occurrence is absent.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn reconcile_familiar_spell<
        F: super::character_build::BuildFormula + Clone + Send + Sync + 'static,
    >(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: FamiliarStateRequest,
        cost: super::spell_items_abi::SpellItemTransactionRequest,
        training: Option<(super::character_build::BuildChangeRequest, F)>,
    ) -> Result<FamiliarSpellCommit> {
        self.commit_familiar_spell_inner(authority, node, fence, request, cost, training, false)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn commit_familiar_spell_inner<
        F: super::character_build::BuildFormula + Clone + Send + Sync + 'static,
    >(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: FamiliarStateRequest,
        cost: super::spell_items_abi::SpellItemTransactionRequest,
        training: Option<(super::character_build::BuildChangeRequest, F)>,
        allow_new_mutation: bool,
    ) -> Result<FamiliarSpellCommit> {
        request.validate()?;
        if cost.command.game_session_id() != fence.game_session_id
            || !cost.operations.is_empty()
            || cost.companion.is_some()
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let binding = command_binding(&fence, &request)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let root = self.clone();
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let item_fence = super::item_transfer::CurrentCharacterItemFence {
                        character_id: fence.character_id,
                        game_session_id: fence.game_session_id,
                        connection_generation: fence.connection_generation,
                        character_lease_generation: fence.character_lease_generation,
                        runtime_scope: fence.runtime_scope,
                        scope_ownership_generation: fence.scope_ownership_generation,
                    };
                    let item_authority =
                        super::spell_item_transaction::assert_spell_item_authority_with_recovery(
                            &mut tx,
                            &root,
                            &recovery,
                            &node,
                            &item_fence,
                            cost.command,
                            cost.catalog_digest,
                        )
                        .await
                        .map_err(item_failure)?;
                    if !allow_new_mutation {
                        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:character-familiar:' || encode($1,'hex'),0))")
                            .bind(request.occurrence.0.as_slice()).execute(&mut *tx).await?;
                        if load_receipt(&mut tx, request.occurrence).await?.is_none() {
                            return Ok(Err(CharacterProgressionError::AuthorityRejected));
                        }
                    }
                    let familiar = match apply_familiar_transaction(
                        &mut tx, &recovery, &node, fence, request, binding, allow_new_mutation,
                    )
                    .await?
                    {
                        Ok(value) => value,
                        Err(error) => return Ok(Err(error)),
                    };
                    let costs = super::spell_item_transaction::apply_spell_items_in_transaction(
                        &mut tx,
                        &item_authority,
                        &cost,
                    )
                    .await
                    .map_err(item_failure)?;
                    let (pending, historical, committed_cost) = match (familiar, costs) {
                        (
                            PendingFamiliarStateOutcome::Prepared(value),
                            super::spell_items_abi::SpellItemTransactionOutcome::Applied(cost),
                        ) => (Some(value), None, cost),
                        (
                            PendingFamiliarStateOutcome::AlreadyCommitted(value),
                            super::spell_items_abi::SpellItemTransactionOutcome::AlreadyCommitted(
                                cost,
                            ),
                        ) => (None, Some(value), cost),
                        _ => return Err(DurabilityError::InvalidStoredState),
                    };
                    let next_revision = pending
                        .as_ref()
                        .map(PendingCharacterFamiliar::committed_character_revision)
                        .or_else(|| {
                            historical
                                .as_ref()
                                .map(CommittedCharacterFamiliar::committed_character_revision)
                        })
                        .ok_or(DurabilityError::InvalidStoredState)?;
                    let mut training_fence = fence;
                    training_fence.expected_character_revision = next_revision;
                    let training = if let Some((request, formula)) = training {
                        let value = super::character_build::prepare_character_build_with_recovery(
                            &mut tx,
                            &recovery,
                            &node,
                            training_fence,
                            request,
                            &formula,
                        )
                        .await;
                        match value {
                            Ok(value) => Some(value),
                            Err(error) => return Ok(Err(error)),
                        }
                    } else {
                        None
                    };
                    if let Some(familiar) = historical {
                        // A pair of independently valid historical rows is insufficient. They
                        // must be the same original physical cast transaction, including any
                        // requested D151 build receipt; replay does not fabricate a new join.
                        let same_transaction: bool = sqlx::query_scalar(
                            "SELECT EXISTS(SELECT 1 FROM game_character_familiar_receipts f JOIN game_spell_item_receipts c ON c.created_xact_id=f.created_xact_id WHERE f.familiar_occurrence_id=encode($1,'hex')::uuid AND c.transaction_id=encode($2,'hex')::uuid)"
                        ).bind(familiar.occurrence().as_bytes().as_slice())
                         .bind(committed_cost.transaction_id.as_slice()).fetch_one(&mut *tx).await?;
                        if !same_transaction { return Err(DurabilityError::InvalidStoredState); }
                        let training = training
                            .map(|value| {
                                value
                                    .historical_outcome()
                                    .ok_or(DurabilityError::InvalidStoredState)
                            })
                            .transpose()?;
                        if let Some(outcome) = &training {
                            let (super::character_build::BuildCommitOutcome::Committed(build)
                                | super::character_build::BuildCommitOutcome::AlreadyCommitted(build)) = outcome;
                            let joined: bool = sqlx::query_scalar(
                                "SELECT EXISTS(SELECT 1 FROM game_character_familiar_receipts f JOIN game_character_build_receipts b ON b.created_xact_id=f.created_xact_id WHERE f.familiar_occurrence_id=encode($1,'hex')::uuid AND b.build_occurrence_id=encode($2,'hex')::uuid)"
                            ).bind(familiar.occurrence().as_bytes().as_slice())
                             .bind(build.occurrence.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                            if !joined { return Err(DurabilityError::InvalidStoredState); }
                        }
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(FamiliarSpellCommit::Reconciled {
                            familiar,
                            cost: committed_cost,
                            training,
                        }));
                    }
                    let pending = super::spell_item_transaction::stage_spell_owner_commit(
                        &mut tx,
                        &item_authority,
                        committed_cost,
                        None,
                        pending,
                        deadline,
                    )
                    .await
                    .map_err(item_failure)?;
                    let common =
                        super::spell_owner_commit::commit_spell_owner_transaction(tx, pending)
                            .await?;
                    let training = training
                        .map(|value| value.after_commit(&common))
                        .transpose()
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    Ok(Ok(FamiliarSpellCommit::Committed { common, training }))
                })
            })
            .await?
    }
}
