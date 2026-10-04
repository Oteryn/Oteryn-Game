//! Explicit ordinary Game-group bootstrap under existing current admission fences.
//! Private snapshots are observed source-qualified data, never renewed admission authority.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::CharacterId;
use crate::foundation::GameSessionId;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};
type Result<T> = std::result::Result<T, CharacterProgressionError>;
const SOURCE: &[u8] =
    include_bytes!("../../../../tools/content-schema/native-gameplay/ordinary-group.json");
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FamiliarGroupSnapshot {
    character: CharacterId,
    source_session: GameSessionId,
    revision: u64,
    source: serde_json::Value,
}
impl FamiliarGroupSnapshot {
    pub(crate) fn character(&self) -> CharacterId {
        self.character
    }
    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }
    pub(crate) fn group_id(&self) -> u32 {
        1
    }
    pub(crate) fn account_type(&self) -> u32 {
        1
    }
    pub(crate) fn access(&self) -> bool {
        false
    }
    pub(crate) fn account_at_least_god(&self) -> bool {
        false
    }
    /// Complete source PlayerFlags namespace is imported. Unknown flag names refuse,
    /// while disabled known flags are explicit membership in that complete source group.
    pub(crate) fn flag(&self, name: &str) -> Option<bool> {
        let name = name.to_ascii_lowercase();
        let known = self.source["known_flags"].as_array()?;
        if !known.iter().any(|v| v.as_str() == Some(&name)) {
            return None;
        }
        Some(
            self.source["enabled_flags"]
                .as_array()?
                .iter()
                .any(|v| v.as_str() == Some(&name)),
        )
    }
}
fn binding(
    character: CharacterId,
    session: GameSessionId,
    source: &serde_json::Value,
) -> std::result::Result<[u8; 32], DurabilityError> {
    Ok(Sha256::new()
        .chain_update(b"oteryn:game-source-group:v1")
        .chain_update(character.as_bytes())
        .chain_update(session.as_bytes())
        .chain_update(serde_json::to_vec(source).map_err(|_| DurabilityError::InvalidStoredState)?)
        .finalize()
        .into())
}
async fn read(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
) -> std::result::Result<Option<FamiliarGroupSnapshot>, DurabilityError> {
    let row=sqlx::query("SELECT r.source_session_id::text,r.revision::text,r.group_id,r.account_type,r.source_document::text,r.semantic_binding FROM game_character_source_group_receipts r JOIN game_character_source_group s USING(character_id) WHERE r.character_id=encode($1,'hex')::uuid AND r.source_session_id=s.source_session_id AND r.revision=s.revision AND r.group_id=s.group_id AND r.account_type=s.account_type")
        .bind(character.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let source: serde_json::Value =
        serde_json::from_str(&row.try_get::<String, _>("source_document")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
    let canonical: serde_json::Value =
        serde_json::from_slice(SOURCE).map_err(|_| DurabilityError::InvalidStoredState)?;
    let revision = row
        .try_get::<String, _>("revision")?
        .parse::<u64>()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if source != canonical
        || revision != 1
        || row.try_get::<i32, _>("group_id")? != 1
        || row.try_get::<i32, _>("account_type")? != 1
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let raw = row
        .try_get::<String, _>("source_session_id")?
        .replace('-', "");
    let mut bytes = [0; 16];
    if raw.len() != 32 {
        return Err(DurabilityError::InvalidStoredState);
    }
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&raw[index * 2..index * 2 + 2], 16)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
    }
    let session = GameSessionId::decode(&bytes).map_err(|_| DurabilityError::InvalidStoredState)?;
    if row.try_get::<Vec<u8>, _>("semantic_binding")?.as_slice()
        != binding(character, session, &source)?
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(Some(FamiliarGroupSnapshot {
        character,
        source_session: session,
        revision,
        source,
    }))
}
/// Same actual transaction as admission initialization. This API accepts no flag or role input.
/// In-transaction initialization is intent until its actual transaction commits.
/// No current administrative producer accepts this pending value; callers read the
/// immutable row under fresh full fences after their observed COMMIT.
#[derive(Debug)]
pub(crate) struct PendingFamiliarGroup {
    snapshot: FamiliarGroupSnapshot,
}
pub(crate) async fn initialize_group_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterGameplayFence,
) -> Result<PendingFamiliarGroup> {
    let recovery = authority
        .record_for(root)
        .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
    assert_recovery_fence(tx, &recovery).await?;
    lock_admission_relations(tx).await?;
    assert_gameplay_fence(tx, fence, node).await??;
    if let Some(value) = read(tx, fence.character_id).await? {
        return Ok(PendingFamiliarGroup { snapshot: value });
    }
    let source: serde_json::Value =
        serde_json::from_slice(SOURCE).map_err(|_| DurabilityError::InvalidStoredState)?;
    let semantic = binding(fence.character_id, fence.game_session_id, &source)?;
    sqlx::query("INSERT INTO game_character_source_group_receipts(source_session_id,character_id,revision,group_id,account_type,source_document,semantic_binding) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,1,$3::jsonb,$4)")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice())
        .bind(serde_json::to_string(&source).map_err(|_|DurabilityError::InvalidStoredState)?).bind(semantic.as_slice()).execute(&mut **tx).await.map_err(DurabilityError::from)?;
    sqlx::query("INSERT INTO game_character_source_group(character_id,revision,source_session_id,group_id,account_type) VALUES(encode($1,'hex')::uuid,1,encode($2,'hex')::uuid,1,1)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).execute(&mut **tx).await.map_err(DurabilityError::from)?;
    read(tx, fence.character_id)
        .await?
        .map(|snapshot| PendingFamiliarGroup { snapshot })
        .ok_or(CharacterProgressionError::BuildStateMismatch)
}
impl DurabilityRoot {
    pub(crate) async fn initialize_familiar_group(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<FamiliarGroupSnapshot> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;
            match assert_gameplay_fence(&mut tx,&fence,&node).await? {Ok(_)=>(),Err(error)=>return Ok(Err(error))};
            let source:serde_json::Value=serde_json::from_slice(SOURCE).map_err(|_|DurabilityError::InvalidStoredState)?;
            if read(&mut tx,fence.character_id).await?.is_none(){
                let semantic=binding(fence.character_id,fence.game_session_id,&source)?;
                sqlx::query("INSERT INTO game_character_source_group_receipts(source_session_id,character_id,revision,group_id,account_type,source_document,semantic_binding) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,1,$3::jsonb,$4)")
                    .bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(serde_json::to_string(&source).map_err(|_|DurabilityError::InvalidStoredState)?).bind(semantic.as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_character_source_group(character_id,revision,source_session_id,group_id,account_type) VALUES(encode($1,'hex')::uuid,1,encode($2,'hex')::uuid,1,1)")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).execute(&mut *tx).await?;
            }
            let value=read(&mut tx,fence.character_id).await?.ok_or(DurabilityError::InvalidStoredState)?;
            commit_semantic_transaction(tx,deadline).await?;Ok(Ok(value))
        })).await?
    }
    pub(crate) async fn read_familiar_group(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<Option<FamiliarGroupSnapshot>> {
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
                    match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        Ok(_) => (),
                        Err(error) => return Ok(Err(error)),
                    };
                    let value = read(&mut tx, fence.character_id).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(value))
                })
            })
            .await?
    }
}

/// Read ordinary group data for an independently current target session in the
/// caller's existing sealed spell transaction. The original group source session
/// is provenance only; it never substitutes for today's admission authority.
pub(crate) async fn read_current_group_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &super::spell_item_transaction::SpellItemAuthority,
    admissions: &super::fresh_admission::FreshAdmissionStore,
    target_session: GameSessionId,
) -> std::result::Result<FamiliarGroupSnapshot, super::spell_item_transaction::SpellItemError> {
    use super::spell_item_transaction::{SpellItemError, check_transaction};
    check_transaction(tx, authority).await?;
    let target = admissions
        .current_session_in_transaction(tx, target_session)
        .await?;
    if target.session_state() == crate::foundation::GameSessionState::Terminal
        || target.commit().game_session_id() != target_session
        || target.current_runtime_scope() != authority.runtime_scope()
        || target.current_scope_generation().get() != authority.scope_generation()
    {
        return Err(SpellItemError::Rejected(
            "target source group session/scope mismatch",
        ));
    }
    let character = CharacterId::from_bytes(*target.commit().character_id().as_bytes())
        .map_err(|_| SpellItemError::Rejected("target source group character identity"))?;
    read(tx, character)
        .await?
        .ok_or(SpellItemError::Rejected("target source group unavailable"))
}

/// Independent admission integrity checks the complete source semantics and binding,
/// not just agreement between stored JSON and its self-consistent hash.
pub(crate) async fn verify_familiar_group(
    tx: &mut Transaction<'_, Postgres>,
) -> std::result::Result<(), DurabilityError> {
    let bad:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_source_group_receipts r FULL JOIN game_character_source_group s USING(character_id) WHERE r.character_id IS NULL OR s.character_id IS NULL OR r.source_session_id<>s.source_session_id OR r.revision<>s.revision OR r.group_id<>s.group_id OR r.account_type<>s.account_type)")
        .fetch_one(&mut **tx).await?;
    if bad {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut after = "00000000-0000-0000-0000-000000000000".to_owned();
    loop {
        let rows:Vec<String>=sqlx::query_scalar("SELECT character_id::text FROM game_character_source_group WHERE character_id>$1::uuid ORDER BY character_id LIMIT 256")
            .bind(&after).fetch_all(&mut **tx).await?;
        let Some(last) = rows.last() else {
            break;
        };
        after = last.clone();
        for raw in rows {
            let hex = raw.replace('-', "");
            let mut bytes = [0; 16];
            if hex.len() != 32 {
                return Err(DurabilityError::InvalidStoredState);
            }
            for (index, byte) in bytes.iter_mut().enumerate() {
                *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
            }
            let character =
                CharacterId::from_bytes(bytes).map_err(|_| DurabilityError::InvalidStoredState)?;
            read(tx, character)
                .await?
                .ok_or(DurabilityError::InvalidStoredState)?;
        }
    }
    Ok(())
}
