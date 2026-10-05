//! STANCE-1 durable standard stance transitions (D145, migration 0017).
//!
//! A successful toggle commits before its runtime stance and costs. It advances the shared
//! CharacterRevision once, leaves XP/build/Monk state untouched, and retains an immutable,
//! fully bound receipt. Replay/reconciliation describe history, never fresh gameplay authority.
//! Actor admission loads the projection; death/logout do not rewrite it. Unknown content keys
//! remain durable and inactive until a qualified replacement toggle uses that stored `before`.

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
use sha2::{Digest, Sha256};
use sqlx::Row;

type Result<T> = std::result::Result<T, CharacterProgressionError>;
const BINDING_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StanceChangeOccurrence([u8; 16]);
impl StanceChangeOccurrence {
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

/// Prepared owner intent. `policy_digest` identifies the qualified toggle policy, not client
/// bytes. Content eligibility and costs are evaluated by the owner before this durable commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StanceChangeRequest {
    pub occurrence: StanceChangeOccurrence,
    pub before: Option<String>,
    pub after: Option<String>,
    pub content_revision: String,
    pub policy_revision: String,
    pub policy_digest: [u8; 32],
}
impl StanceChangeRequest {
    fn validate(&self) -> Result<()> {
        if self.before == self.after
            || !self.before.as_deref().is_none_or(valid_revision)
            || !self.after.as_deref().is_none_or(valid_revision)
            || !valid_revision(&self.content_revision)
            || !valid_revision(&self.policy_revision)
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DurableCharacterStance {
    key: Option<String>,
    committed_character_revision: Option<CharacterRevision>,
}
impl DurableCharacterStance {
    #[must_use]
    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }
    #[must_use]
    pub const fn committed_character_revision(&self) -> Option<CharacterRevision> {
        self.committed_character_revision
    }
}

/// Only the durable writer/reader constructs this proof of the historical commit. Callers must
/// also independently own current actor/session authority before applying it to runtime state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedCharacterStance {
    occurrence: StanceChangeOccurrence,
    character_id: CharacterId,
    original_character_revision: CharacterRevision,
    committed_character_revision: CharacterRevision,
    before: Option<String>,
    after: Option<String>,
    content_revision: String,
    policy_revision: String,
    policy_digest: [u8; 32],
    binding: Vec<u8>,
}
impl CommittedCharacterStance {
    #[must_use]
    pub const fn occurrence(&self) -> StanceChangeOccurrence {
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
    pub fn before(&self) -> Option<&str> {
        self.before.as_deref()
    }
    #[must_use]
    pub fn after(&self) -> Option<&str> {
        self.after.as_deref()
    }
    #[must_use]
    pub fn content_revision(&self) -> &str {
        &self.content_revision
    }
    #[must_use]
    pub fn policy_revision(&self) -> &str {
        &self.policy_revision
    }
    #[must_use]
    pub const fn policy_digest(&self) -> &[u8; 32] {
        &self.policy_digest
    }
    /// A retained receipt may be applied only to this exact prepared intent. This check alone
    /// does not authorize mutation; current authority and actor revision still belong to owner.
    #[must_use]
    pub fn matches_request(
        &self,
        fence: &CurrentCharacterGameplayFence,
        request: &StanceChangeRequest,
    ) -> bool {
        request.validate().is_ok()
            && self.character_id == fence.character_id
            && self.original_character_revision == fence.expected_character_revision
            && self.occurrence == request.occurrence
            && self.before == request.before
            && self.after == request.after
            && self.content_revision == request.content_revision
            && self.policy_revision == request.policy_revision
            && self.policy_digest == request.policy_digest
            && self.binding == command_binding(fence, request)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StanceChangeOutcome {
    Committed(CommittedCharacterStance),
    AlreadyCommitted(CommittedCharacterStance),
}

impl DurabilityRoot {
    pub async fn commit_character_stance(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: StanceChangeRequest,
    ) -> Result<StanceChangeOutcome> {
        request.validate()?;
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
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:character-stance:' || encode($1,'hex'),0))")
                .bind(request.occurrence.0.as_slice()).execute(&mut *tx).await?;
            // Replay precedes current-session checks, but still requires the current recovery seal.
            if let Some(row) = load_receipt(&mut tx, request.occurrence).await? {
                let stored: Vec<u8> = row.try_get("command_binding")?;
                if stored != binding { return Ok(Err(CharacterProgressionError::ConflictingOccurrence)); }
                let receipt = decode_receipt(&row)?;
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(Ok(StanceChangeOutcome::AlreadyCommitted(receipt)));
            }
            let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                Ok(root) => root, Err(error) => return Ok(Err(error)),
            };
            let state = sqlx::query("SELECT character_revision::text, profile_revision, ruleset_revision, content_revision, policy_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let Some(state) = state else { return Ok(Err(CharacterProgressionError::MissingProgressionState)); };
            if numeric_u64(&state,"character_revision")? != root.revision { return Err(DurabilityError::InvalidStoredState); }
            if !state_matches_root(&state, &root)
                || state.try_get::<String,_>("content_revision")? != request.content_revision
                || state.try_get::<String,_>("policy_revision")? != request.policy_revision
            { return Ok(Err(CharacterProgressionError::ProgressionContextMismatch)); }
            let projection = sqlx::query("SELECT stance_key, committed_character_revision::text FROM game_character_stance WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let before = projection.as_ref().map(decode_projection).transpose()?.unwrap_or_default();
            if before.committed_character_revision.is_some_and(|revision| revision.get() > root.revision) {
                return Err(DurabilityError::InvalidStoredState);
            }
            if before.key != request.before { return Ok(Err(CharacterProgressionError::StanceStateMismatch)); }
            let pending: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_pending_respawns WHERE character_id=encode($1,'hex')::uuid)")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            if pending { return Ok(Err(CharacterProgressionError::RespawnPending)); }
            let next = root.revision.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
            for query in [
                "UPDATE game_character_roots SET character_revision=$2::text::numeric(20,0) WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric(20,0)",
                "UPDATE game_character_progression_state SET character_revision=$2::text::numeric(20,0) WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric(20,0)",
            ] {
                if sqlx::query(query).bind(fence.character_id.as_bytes().as_slice()).bind(next.to_string()).bind(root.revision.to_string()).execute(&mut *tx).await?.rows_affected() != 1 {
                    return Err(DurabilityError::InvalidStoredState);
                }
            }
            let inserted = sqlx::query("INSERT INTO game_character_stance_receipts(stance_occurrence_id,command_binding,policy_digest,character_id,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,stance_before,stance_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,committed_at) SELECT encode($1,'hex')::uuid,$2,$3,s.character_id,$4::text::numeric(20,0),$5::text::numeric(20,0),s.level,s.level,s.total_experience,s.total_experience,$6,$7,s.profile_revision,s.ruleset_revision,s.content_revision,s.simulation_revision,s.evidence_revision,s.declaration_revision,s.policy_revision,s.reward_revision,floor(extract(epoch FROM statement_timestamp())*1000)::bigint FROM game_character_progression_state s WHERE s.character_id=encode($8,'hex')::uuid")
                .bind(request.occurrence.0.as_slice()).bind(&binding).bind(request.policy_digest.as_slice())
                .bind(root.revision.to_string()).bind(next.to_string()).bind(request.before.as_deref()).bind(request.after.as_deref())
                .bind(fence.character_id.as_bytes().as_slice()).execute(&mut *tx).await?;
            if inserted.rows_affected() != 1 { return Err(DurabilityError::InvalidStoredState); }
            let updated = sqlx::query("INSERT INTO game_character_stance(character_id,stance_key,committed_character_revision,last_stance_occurrence_id) VALUES(encode($1,'hex')::uuid,$2,$3::text::numeric(20,0),encode($4,'hex')::uuid) ON CONFLICT(character_id) DO UPDATE SET stance_key=EXCLUDED.stance_key,committed_character_revision=EXCLUDED.committed_character_revision,last_stance_occurrence_id=EXCLUDED.last_stance_occurrence_id")
                .bind(fence.character_id.as_bytes().as_slice()).bind(request.after.as_deref()).bind(next.to_string()).bind(request.occurrence.0.as_slice()).execute(&mut *tx).await?;
            if updated.rows_affected() != 1 { return Err(DurabilityError::InvalidStoredState); }
            let receipt = CommittedCharacterStance {
                occurrence:request.occurrence, character_id:fence.character_id,
                original_character_revision:fence.expected_character_revision,
                committed_character_revision:CharacterRevision::new(next).map_err(|_| DurabilityError::InvalidStoredState)?,
                before:request.before, after:request.after, content_revision:request.content_revision,
                policy_revision:request.policy_revision, policy_digest:request.policy_digest, binding,
            };
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Ok(StanceChangeOutcome::Committed(receipt)))
        })).await?
    }

    pub async fn reconcile_character_stance(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        occurrence: StanceChangeOccurrence,
    ) -> Result<Option<CommittedCharacterStance>> {
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

    pub async fn read_character_stance(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character_id: CharacterId,
    ) -> Result<DurableCharacterStance> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?.run(move |holder,deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            let row = sqlx::query("SELECT stance_key,committed_character_revision::text FROM game_character_stance WHERE character_id=encode($1,'hex')::uuid")
                .bind(character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let projection = row.as_ref().map(decode_projection).transpose()?.unwrap_or_default();
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Ok(projection))
        })).await?
    }
}

fn command_binding(
    fence: &CurrentCharacterGameplayFence,
    request: &StanceChangeRequest,
) -> Vec<u8> {
    fn text(semantic: &mut Vec<u8>, value: &str) {
        semantic.extend_from_slice(&(value.len() as u64).to_be_bytes());
        semantic.extend_from_slice(value.as_bytes());
    }
    fn key(semantic: &mut Vec<u8>, value: Option<&str>) {
        semantic.push(u8::from(value.is_some()));
        if let Some(value) = value {
            text(semantic, value);
        }
    }
    let mut semantic = vec![BINDING_VERSION];
    semantic.extend_from_slice(&request.occurrence.0);
    semantic.extend_from_slice(fence.character_id.as_bytes());
    semantic.extend_from_slice(&fence.expected_character_revision.get().to_be_bytes());
    key(&mut semantic, request.before.as_deref());
    key(&mut semantic, request.after.as_deref());
    text(&mut semantic, &request.content_revision);
    text(&mut semantic, &request.policy_revision);
    semantic.extend_from_slice(&request.policy_digest);
    let mut binding = vec![BINDING_VERSION];
    binding.extend_from_slice(&Sha256::digest(&semantic));
    binding
}
async fn load_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    occurrence: StanceChangeOccurrence,
) -> std::result::Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(sqlx::query("SELECT stance_occurrence_id::text,command_binding,policy_digest,character_id::text,original_character_revision::text,committed_character_revision::text,stance_before,stance_after,content_revision,policy_revision FROM game_character_stance_receipts WHERE stance_occurrence_id=encode($1,'hex')::uuid")
        .bind(occurrence.0.as_slice()).fetch_optional(&mut **tx).await?)
}
fn stored_key(value: Option<String>) -> std::result::Result<Option<String>, DurabilityError> {
    if value.as_deref().is_none_or(valid_revision) {
        Ok(value)
    } else {
        Err(DurabilityError::InvalidStoredState)
    }
}
fn decode_projection(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<DurableCharacterStance, DurabilityError> {
    let revision = numeric_u64(row, "committed_character_revision")?;
    if revision < 2 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(DurableCharacterStance {
        key: stored_key(row.try_get("stance_key")?)?,
        committed_character_revision: Some(
            CharacterRevision::new(revision).map_err(|_| DurabilityError::InvalidStoredState)?,
        ),
    })
}
fn decode_receipt(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<CommittedCharacterStance, DurabilityError> {
    let original = CharacterRevision::new(numeric_u64(row, "original_character_revision")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let committed = CharacterRevision::new(numeric_u64(row, "committed_character_revision")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let before = stored_key(row.try_get("stance_before")?)?;
    let after = stored_key(row.try_get("stance_after")?)?;
    let content_revision: String = row.try_get("content_revision")?;
    let policy_revision: String = row.try_get("policy_revision")?;
    let binding: Vec<u8> = row.try_get("command_binding")?;
    let policy_digest: Vec<u8> = row.try_get("policy_digest")?;
    if original.get().checked_add(1) != Some(committed.get())
        || before == after
        || !valid_revision(&content_revision)
        || !valid_revision(&policy_revision)
        || binding.len() != 33
        || binding[0] != BINDING_VERSION
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(CommittedCharacterStance {
        occurrence: StanceChangeOccurrence::from_bytes(uuid_text(
            row.try_get("stance_occurrence_id")?,
        )?)
        .map_err(|_| DurabilityError::InvalidStoredState)?,
        character_id: CharacterId::from_bytes(uuid_text(row.try_get("character_id")?)?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        original_character_revision: original,
        committed_character_revision: committed,
        before,
        after,
        content_revision,
        policy_revision,
        policy_digest: policy_digest
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        binding,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::foundation::{
        ChannelId, ConnectionGeneration, GameSessionId, ScopeOwnershipGeneration, WorldId,
    };
    fn id(n: u8) -> [u8; 16] {
        [n, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, n]
    }
    fn fence() -> CurrentCharacterGameplayFence {
        CurrentCharacterGameplayFence {
            character_id: CharacterId::from_bytes(id(1)).expect("valid fixture"),
            game_session_id: GameSessionId::decode(&id(2)).expect("valid fixture"),
            connection_generation: ConnectionGeneration::new(1).expect("valid fixture"),
            character_lease_generation: 1,
            runtime_scope: RuntimeScopeRefV1::Channel {
                world_id: WorldId::decode(&id(3)).expect("valid fixture"),
                channel_id: ChannelId::decode(&id(4)).expect("valid fixture"),
            },
            scope_ownership_generation: ScopeOwnershipGeneration::new(1).expect("valid fixture"),
            expected_character_revision: CharacterRevision::new(1).expect("valid fixture"),
        }
    }
    fn request() -> StanceChangeRequest {
        StanceChangeRequest {
            occurrence: StanceChangeOccurrence::from_bytes(id(5)).expect("valid fixture"),
            before: None,
            after: Some("protector".into()),
            content_revision: "content-1".into(),
            policy_revision: "policy-1".into(),
            policy_digest: [1; 32],
        }
    }
    #[test]
    fn stance_keys_noops_and_occurrences_fail_closed() {
        assert!(StanceChangeOccurrence::from_bytes([7; 16]).is_err());
        for invalid in ["", "-key", "a b", "a/b", &"a".repeat(129)] {
            let mut r = request();
            r.after = Some(invalid.into());
            assert!(r.validate().is_err());
        }
        let mut r = request();
        r.before = r.after.clone();
        assert!(r.validate().is_err());
        r.before = None;
        r.after = None;
        assert!(r.validate().is_err());
        let mut r = request();
        r.before = Some("removed-content-key".into());
        assert!(r.validate().is_ok());
    }
    #[test]
    fn full_intent_is_bound_but_retry_local_authority_is_not() {
        let f = fence();
        let r = request();
        let binding = command_binding(&f, &r);
        let mut retry = f;
        retry.connection_generation = ConnectionGeneration::new(2).expect("valid fixture");
        retry.character_lease_generation = 2;
        retry.scope_ownership_generation = ScopeOwnershipGeneration::new(2).expect("valid fixture");
        retry.game_session_id = GameSessionId::decode(&id(6)).expect("valid fixture");
        assert_eq!(binding, command_binding(&retry, &r));
        let mut changed = request();
        changed.before = Some("blood-rage".into());
        assert_ne!(binding, command_binding(&f, &changed));
        changed = request();
        changed.after = None;
        assert_ne!(binding, command_binding(&f, &changed));
        changed = request();
        changed.content_revision = "content-2".into();
        assert_ne!(binding, command_binding(&f, &changed));
        changed = request();
        changed.policy_revision = "policy-2".into();
        assert_ne!(binding, command_binding(&f, &changed));
        changed = request();
        changed.policy_digest = [2; 32];
        assert_ne!(binding, command_binding(&f, &changed));
        changed = request();
        changed.occurrence = StanceChangeOccurrence::from_bytes(id(7)).expect("valid fixture");
        assert_ne!(binding, command_binding(&f, &changed));
        retry = f;
        retry.expected_character_revision = CharacterRevision::new(2).expect("valid fixture");
        assert_ne!(binding, command_binding(&retry, &r));
        retry = f;
        retry.character_id = CharacterId::from_bytes(id(8)).expect("valid fixture");
        assert_ne!(binding, command_binding(&retry, &r));
    }
}
