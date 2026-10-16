//! APP-1 native durable selection writer. Production composition requires the CP integration
//! slot and the real content/Character-sex owners. Earned/Store/mount authorization stays closed
//! until its accepted resolver/consumer exists. Historical receipts never grant actor authority.
//!
//! The allocated companion source holds the one reader/recovery implementation and its tests;
//! this writer is compiled unconditionally whenever the native owner registers this module.

#[path = "character_appearance_tests.rs"]
pub(crate) mod stored;

pub(crate) use stored::{
    AppearanceReadError, DurableAppearanceSelection, read_selected_appearance,
    verify_character_appearance_integrity,
};
// Preserve the original account-read seam while its runtime consumer awaits CP integration.
#[allow(unused_imports)]
pub(crate) use stored::{AccountAppearanceUnlock, read_account_appearance_unlocks};
use stored::{decode_selection_fields, valid_provenance};

use crate::domain::appearance::AppearanceSelection;
use crate::domain::{CharacterId, CharacterRevision};
use sqlx::Row;
use sqlx::postgres::PgRow;

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
use crate::domain::appearance::{
    AccountUnlocks, AppearanceContent, AppearanceRejection, Sex, UnlockFact, validate_selection,
};
use crate::foundation::RuntimeScopeRefV1;
use sha2::{Digest, Sha256};

const BINDING_VERSION: u8 = 1;

#[derive(Debug)]
pub(crate) enum AppearanceWriteError {
    InvalidInput,
    StateMismatch,
    Rejected(AppearanceRejection),
    Fence(CharacterProgressionError),
    Unavailable(DurabilityError),
}
impl From<DurabilityError> for AppearanceWriteError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}
impl From<CharacterProgressionError> for AppearanceWriteError {
    fn from(error: CharacterProgressionError) -> Self {
        Self::Fence(error)
    }
}
impl std::fmt::Display for AppearanceWriteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "appearance transition rejected: {self:?}")
    }
}
impl std::error::Error for AppearanceWriteError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AppearanceChangeOccurrence([u8; 16]);
impl AppearanceChangeOccurrence {
    pub(crate) fn from_bytes(bytes: [u8; 16]) -> Result<Self, AppearanceWriteError> {
        if bytes[6] >> 4 != 7 || bytes[8] & 0xc0 != 0x80 {
            return Err(AppearanceWriteError::InvalidInput);
        }
        Ok(Self(bytes))
    }
    #[must_use]
    pub(crate) const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

/// Prepared intent, not a validation or current authority proof. `before` retains its original
/// content provenance; an incompatible historical selection can be replaced without rewriting it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AppearanceChangeRequest {
    pub(crate) occurrence: AppearanceChangeOccurrence,
    pub(crate) before: Option<DurableAppearanceSelection>,
    pub(crate) after: AppearanceSelection<String>,
    pub(crate) content_revision: String,
}
impl AppearanceChangeRequest {
    fn validate(&self) -> Result<(), AppearanceWriteError> {
        if !valid_provenance(&self.content_revision)
            || self
                .before
                .as_ref()
                .is_some_and(|before| !valid_provenance(&before.content_revision))
            || self
                .before
                .as_ref()
                .is_some_and(|before| before.selection == self.after)
        {
            return Err(AppearanceWriteError::InvalidInput);
        }
        Ok(())
    }
}

/// Historical disposition only. Neither replay nor reconciliation authorizes actor publication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommittedCharacterAppearance {
    pub(crate) occurrence: AppearanceChangeOccurrence,
    pub(crate) character_id: CharacterId,
    pub(crate) original_character_revision: CharacterRevision,
    pub(crate) committed_character_revision: CharacterRevision,
    pub(crate) before: Option<DurableAppearanceSelection>,
    pub(crate) after: AppearanceSelection<String>,
    pub(crate) content_revision: String,
    binding: Vec<u8>,
}
impl CommittedCharacterAppearance {
    #[must_use]
    pub(crate) fn matches_request(
        &self,
        fence: &CurrentCharacterGameplayFence,
        request: &AppearanceChangeRequest,
        sex: Sex,
    ) -> bool {
        request.validate().is_ok()
            && self.occurrence == request.occurrence
            && self.character_id == fence.character_id
            && self.original_character_revision == fence.expected_character_revision
            && self.before == request.before
            && self.after == request.after
            && self.content_revision == request.content_revision
            && self.binding == appearance_binding(fence, request, sex)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AppearanceChangeOutcome {
    Committed(CommittedCharacterAppearance),
    AlreadyCommitted(CommittedCharacterAppearance),
}

/// Pure, independently loaded interpretation input; it is not a session or grant proof.
/// Production composition must derive sex from Character creation and content from the exact
/// active owner generation. Until those owners supply it there is no runtime caller.
pub(crate) struct AppearanceValidationContext<C> {
    pub(crate) sex: Sex,
    pub(crate) content: C,
    pub(crate) content_revision: String,
}

// The accepted content mapping for earned provenance/addon keys does not exist yet. Keep that
// entire authorization path unavailable instead of taking an authorizing callback or guessing
// a content-key convention. Native starter semantics still pass through the accepted validator.
struct UnavailableEarnedUnlocks;
impl AccountUnlocks<String> for UnavailableEarnedUnlocks {
    fn outfit(&self, _: &String) -> UnlockFact {
        UnlockFact::Absent
    }
    fn addon(&self, _: &String, _: u8) -> UnlockFact {
        UnlockFact::Absent
    }
    fn mount(&self, _: &String) -> UnlockFact {
        UnlockFact::Absent
    }
}

impl DurabilityRoot {
    /// Commit the selection under the native Character fence. The owner supplies its current
    /// immutable content view and independently resolved Character sex, never request authority.
    /// Earned outfits/addons/mounts and Store remain unavailable: no accepted content resolver
    /// or granting consumer is fabricated. No grant authority is accepted here.
    ///
    /// Production composition waits for APP-2's real mapping/compatibility resolver, Character
    /// sex input, and the CP migration/integrity/module-registration slot.
    pub(crate) async fn commit_character_appearance<C>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: AppearanceChangeRequest,
        validation: AppearanceValidationContext<C>,
    ) -> Result<AppearanceChangeOutcome, AppearanceWriteError>
    where
        C: AppearanceContent<String> + Send + 'static,
    {
        let AppearanceValidationContext {
            sex,
            content,
            content_revision,
        } = validation;
        request.validate()?;
        if fence.character_lease_generation == 0
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        {
            return Err(AppearanceWriteError::InvalidInput);
        }
        let binding = appearance_binding(&fence, &request, sex);
        let recovery = authority.record_for(self).map_err(|_| {
            AppearanceWriteError::Fence(CharacterProgressionError::AuthorityRejected)
        })?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:character-appearance:' || encode($1,'hex'),0))")
                .bind(request.occurrence.0.as_slice()).execute(&mut *tx).await?;
            if let Some(row) = load_appearance_receipt(&mut tx, request.occurrence).await? {
                if row.try_get::<Vec<u8>,_>("command_binding")? != binding {
                    return Ok(Err(AppearanceWriteError::Fence(CharacterProgressionError::ConflictingOccurrence)));
                }
                let receipt = decode_appearance_receipt(&row)?;
                if !receipt.matches_request(&fence, &request, sex) { return Err(DurabilityError::InvalidStoredState); }
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(Ok(AppearanceChangeOutcome::AlreadyCommitted(receipt)));
            }
            let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                Ok(root) => root, Err(error) => return Ok(Err(error.into())),
            };
            let state = sqlx::query("SELECT character_revision::text, profile_revision, ruleset_revision, content_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let Some(state) = state else { return Ok(Err(CharacterProgressionError::MissingProgressionState.into())); };
            if numeric_u64(&state, "character_revision")? != root.revision { return Err(DurabilityError::InvalidStoredState); }
            if !state_matches_root(&state, &root) || state.try_get::<String,_>("content_revision")? != request.content_revision || content_revision != root.content_revision {
                return Ok(Err(CharacterProgressionError::ProgressionContextMismatch.into()));
            }
            let before = read_selected_appearance(&mut tx, fence.character_id).await.map_err(read_storage_error)?;
            if before.as_ref().is_some_and(|before| before.committed_character_revision.get() > root.revision) { return Err(DurabilityError::InvalidStoredState); }
            if before != request.before { return Ok(Err(AppearanceWriteError::StateMismatch)); }
            let pending: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_pending_respawns WHERE character_id=encode($1,'hex')::uuid)")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            if pending { return Ok(Err(CharacterProgressionError::RespawnPending.into())); }
            if let Err(rejection) = validate_selection(&request.after, sex, &content, &UnavailableEarnedUnlocks) {
                return Ok(Err(AppearanceWriteError::Rejected(rejection)));
            }
            let next = root.revision.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
            for query in [
                "UPDATE game_character_roots SET character_revision=$2::text::numeric(20,0) WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric(20,0)",
                "UPDATE game_character_progression_state SET character_revision=$2::text::numeric(20,0) WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric(20,0)",
            ] {
                if sqlx::query(query).bind(fence.character_id.as_bytes().as_slice()).bind(next.to_string()).bind(root.revision.to_string()).execute(&mut *tx).await?.rows_affected() != 1 { return Err(DurabilityError::InvalidStoredState); }
            }
            let before = request.before.as_ref();
            let before_colours = before.map(|before| before.selection.colours.map(i32::from).to_vec());
            let after_colours = request.after.colours.map(i32::from).to_vec();
            let inserted = sqlx::query("INSERT INTO game_character_appearance_receipts(appearance_occurrence_id,command_binding,character_id,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,outfit_before,colours_before,addons_before,mount_before,content_before,selection_revision_before,outfit_after,colours_after,addons_after,mount_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,committed_at) SELECT encode($1,'hex')::uuid,$2,s.character_id,$3::text::numeric(20,0),$4::text::numeric(20,0),s.level,s.level,s.total_experience,s.total_experience,$5,$6,$7,$8,$9,$10::text::numeric(20,0),$11,$12,$13,$14,s.profile_revision,s.ruleset_revision,s.content_revision,s.simulation_revision,s.evidence_revision,s.declaration_revision,s.policy_revision,s.reward_revision,floor(extract(epoch FROM statement_timestamp())*1000)::bigint FROM game_character_progression_state s WHERE s.character_id=encode($15,'hex')::uuid")
                .bind(request.occurrence.0.as_slice()).bind(&binding).bind(root.revision.to_string()).bind(next.to_string())
                .bind(before.map(|b| b.selection.outfit_key.as_str())).bind(before_colours).bind(before.map(|b| i16::from(b.selection.addons)))
                .bind(before.and_then(|b| b.selection.mount_key.as_deref())).bind(before.map(|b| b.content_revision.as_str()))
                .bind(before.map(|b| b.committed_character_revision.get().to_string()))
                .bind(&request.after.outfit_key).bind(&after_colours).bind(i16::from(request.after.addons)).bind(request.after.mount_key.as_deref())
                .bind(fence.character_id.as_bytes().as_slice()).execute(&mut *tx).await?;
            if inserted.rows_affected() != 1 { return Err(DurabilityError::InvalidStoredState); }
            let updated = sqlx::query("INSERT INTO game_character_appearance(character_id,outfit_key,colours,addons,mount_key,content_revision,committed_character_revision,last_appearance_occurrence_id) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5,$6,$7::text::numeric(20,0),encode($8,'hex')::uuid) ON CONFLICT(character_id) DO UPDATE SET outfit_key=EXCLUDED.outfit_key,colours=EXCLUDED.colours,addons=EXCLUDED.addons,mount_key=EXCLUDED.mount_key,content_revision=EXCLUDED.content_revision,committed_character_revision=EXCLUDED.committed_character_revision,last_appearance_occurrence_id=EXCLUDED.last_appearance_occurrence_id")
                .bind(fence.character_id.as_bytes().as_slice()).bind(&request.after.outfit_key).bind(after_colours).bind(i16::from(request.after.addons))
                .bind(request.after.mount_key.as_deref()).bind(&request.content_revision).bind(next.to_string()).bind(request.occurrence.0.as_slice())
                .execute(&mut *tx).await?;
            if updated.rows_affected() != 1 { return Err(DurabilityError::InvalidStoredState); }
            let receipt = CommittedCharacterAppearance {
                occurrence: request.occurrence, character_id: fence.character_id,
                original_character_revision: fence.expected_character_revision,
                committed_character_revision: CharacterRevision::new(next).map_err(|_| DurabilityError::InvalidStoredState)?,
                before: request.before, after: request.after, content_revision: request.content_revision, binding,
            };
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(AppearanceChangeOutcome::Committed(receipt)))
        })).await?
    }

    /// Read stored selection DATA under the current native DB gameplay fence.
    /// This proves neither active content/sex compatibility nor current runtime actor/condition
    /// ownership. Callers must independently recheck those owners before publication; selection
    /// provenance may deliberately differ from the currently admitted content revision.
    pub(crate) async fn read_current_character_appearance(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<Option<DurableAppearanceSelection>, AppearanceWriteError> {
        if fence.character_lease_generation == 0
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        {
            return Err(AppearanceWriteError::InvalidInput);
        }
        let recovery = authority.record_for(self).map_err(|_| {
            AppearanceWriteError::Fence(CharacterProgressionError::AuthorityRejected)
        })?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => root,
                        Err(error) => return Ok(Err(error.into())),
                    };
                    let state = sqlx::query("SELECT character_revision::text, profile_revision, ruleset_revision, content_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                        .bind(fence.character_id.as_bytes().as_slice())
                        .fetch_optional(&mut *tx).await?;
                    let Some(state) = state else {
                        return Ok(Err(CharacterProgressionError::MissingProgressionState.into()));
                    };
                    if numeric_u64(&state, "character_revision")? != root.revision {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    if !state_matches_root(&state, &root) {
                        return Ok(Err(CharacterProgressionError::ProgressionContextMismatch.into()));
                    }
                    let selected = read_selected_appearance(&mut tx, fence.character_id)
                        .await.map_err(read_storage_error)?;
                    if selected.as_ref().is_some_and(|selected| selected.committed_character_revision.get() > root.revision) {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let pending: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_pending_respawns WHERE character_id=encode($1,'hex')::uuid)")
                        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                    if pending {
                        return Ok(Err(CharacterProgressionError::RespawnPending.into()));
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(selected))
                })
            })
            .await?
    }

    pub(crate) async fn read_character_appearance(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
    ) -> Result<Option<DurableAppearanceSelection>, AppearanceWriteError> {
        let recovery = authority.record_for(self).map_err(|_| {
            AppearanceWriteError::Fence(CharacterProgressionError::AuthorityRejected)
        })?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let selected = read_selected_appearance(&mut tx, character)
                        .await
                        .map_err(read_storage_error)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(selected))
                })
            })
            .await?
    }

    pub(crate) async fn reconcile_character_appearance(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        occurrence: AppearanceChangeOccurrence,
    ) -> Result<Option<CommittedCharacterAppearance>, AppearanceWriteError> {
        let recovery = authority.record_for(self).map_err(|_| {
            AppearanceWriteError::Fence(CharacterProgressionError::AuthorityRejected)
        })?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let receipt = load_appearance_receipt(&mut tx, occurrence)
                        .await?
                        .as_ref()
                        .map(decode_appearance_receipt)
                        .transpose()?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(receipt))
                })
            })
            .await?
    }
}

fn read_storage_error(error: AppearanceReadError) -> DurabilityError {
    match error {
        AppearanceReadError::InvalidStoredState => DurabilityError::InvalidStoredState,
        AppearanceReadError::Storage(error) => error.into(),
    }
}

fn appearance_binding(
    fence: &CurrentCharacterGameplayFence,
    request: &AppearanceChangeRequest,
    sex: Sex,
) -> Vec<u8> {
    fn text(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u64).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
    fn selection(out: &mut Vec<u8>, value: &AppearanceSelection<String>) {
        text(out, &value.outfit_key);
        for colour in value.colours {
            out.extend_from_slice(&colour.to_be_bytes());
        }
        out.push(value.addons);
        out.push(u8::from(value.mount_key.is_some()));
        if let Some(mount) = &value.mount_key {
            text(out, mount);
        }
    }
    let mut semantic = vec![BINDING_VERSION];
    semantic.extend_from_slice(&request.occurrence.0);
    semantic.extend_from_slice(fence.character_id.as_bytes());
    semantic.extend_from_slice(&fence.expected_character_revision.get().to_be_bytes());
    semantic.push(match sex {
        Sex::Female => 0,
        Sex::Male => 1,
    });
    semantic.push(u8::from(request.before.is_some()));
    if let Some(before) = &request.before {
        selection(&mut semantic, &before.selection);
        text(&mut semantic, &before.content_revision);
        semantic.extend_from_slice(&before.committed_character_revision.get().to_be_bytes());
    }
    selection(&mut semantic, &request.after);
    text(&mut semantic, &request.content_revision);
    let mut binding = vec![BINDING_VERSION];
    binding.extend_from_slice(&Sha256::digest(&semantic));
    binding
}

async fn load_appearance_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    occurrence: AppearanceChangeOccurrence,
) -> Result<Option<PgRow>, DurabilityError> {
    Ok(sqlx::query("SELECT appearance_occurrence_id::text,command_binding,character_id::text,original_character_revision::text,committed_character_revision::text,outfit_before,colours_before,addons_before,mount_before,content_before,selection_revision_before::text,outfit_after,colours_after,addons_after,mount_after,content_revision FROM game_character_appearance_receipts WHERE appearance_occurrence_id=encode($1,'hex')::uuid")
        .bind(occurrence.0.as_slice()).fetch_optional(&mut **tx).await?)
}
fn decode_appearance_receipt(row: &PgRow) -> Result<CommittedCharacterAppearance, DurabilityError> {
    let original = numeric_u64(row, "original_character_revision")?;
    let committed = numeric_u64(row, "committed_character_revision")?;
    let outfit_before: Option<String> = row.try_get("outfit_before")?;
    let colours_before: Option<Vec<i32>> = row.try_get("colours_before")?;
    let addons_before: Option<i16> = row.try_get("addons_before")?;
    let content_before: Option<String> = row.try_get("content_before")?;
    let revision_before: Option<String> = row.try_get("selection_revision_before")?;
    let mount_before: Option<String> = row.try_get("mount_before")?;
    let before = match (
        outfit_before,
        colours_before,
        addons_before,
        content_before,
        revision_before,
    ) {
        (Some(outfit), Some(colours), Some(addons), Some(content), Some(revision)) => Some(
            decode_selection_fields(outfit, &colours, addons, mount_before, content, &revision)
                .map_err(read_storage_error)?,
        ),
        (None, None, None, None, None) if mount_before.is_none() => None,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let content_revision: String = row.try_get("content_revision")?;
    let after = decode_selection_fields(
        row.try_get("outfit_after")?,
        &row.try_get::<Vec<i32>, _>("colours_after")?,
        row.try_get("addons_after")?,
        row.try_get("mount_after")?,
        content_revision.clone(),
        &committed.to_string(),
    )
    .map_err(read_storage_error)?
    .selection;
    let binding: Vec<u8> = row.try_get("command_binding")?;
    if original.checked_add(1) != Some(committed)
        || original == 0
        || before.as_ref().is_some_and(|before| {
            before.selection == after || before.committed_character_revision.get() > original
        })
        || binding.len() != 33
        || binding[0] != BINDING_VERSION
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(CommittedCharacterAppearance {
        occurrence: AppearanceChangeOccurrence::from_bytes(uuid_text(
            row.try_get("appearance_occurrence_id")?,
        )?)
        .map_err(|_| DurabilityError::InvalidStoredState)?,
        character_id: CharacterId::from_bytes(uuid_text(row.try_get("character_id")?)?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        original_character_revision: CharacterRevision::new(original)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        committed_character_revision: CharacterRevision::new(committed)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        before,
        after,
        content_revision,
        binding,
    })
}
