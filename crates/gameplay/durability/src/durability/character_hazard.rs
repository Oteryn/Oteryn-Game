//! Current-session fenced hazard selection. Clients never supply unlocked levels.
mod house_read;
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence, numeric_u64,
    state_matches_root,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use sqlx::Row;

mod primal;
pub use primal::apply_hazard_kill_in_transaction;

impl DurabilityRoot {
    /// Read or select an unlocked level, under the same fence and transaction.
    pub async fn character_hazard(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: Option<(u64, u16)>,
    ) -> Result<Option<(u16, u16, Vec<u8>)>, CharacterProgressionError> {
        if request.is_some_and(|(id, level)| id == 0 || !(1..=12).contains(&level)) {
            return Ok(None);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? { Ok(root) => root, Err(error) => return Ok(Err(error)) };
            sqlx::query("INSERT INTO game_character_hazard(character_id) VALUES(encode($1,'hex')::uuid) ON CONFLICT DO NOTHING")
                .bind(fence.character_id.as_bytes().as_slice()).execute(&mut *tx).await?;
            let (unlocked, current): (i16, i16) = sqlx::query_as("SELECT unlocked,selected FROM game_character_hazard WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            let mut response = vec![unlocked as u8, current as u8];
            let mut selected = current as u16;
            if let Some((id, requested)) = request {
                if let Some(receipt) = sqlx::query("SELECT selected_requested,result FROM game_character_hazard_receipts WHERE character_id=encode($1,'hex')::uuid AND command_session_id=encode($2,'hex')::uuid AND connection_generation=$3::text::numeric AND command_id=$4::text::numeric")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice())
                    .bind(fence.connection_generation.get().to_string()).bind(id.to_string()).fetch_optional(&mut *tx).await? {
                    if receipt.try_get::<i16,_>("selected_requested")? != requested as i16 { return Ok(Err(CharacterProgressionError::ConflictingOccurrence)); }
                    response = receipt.try_get("result")?;
                } else {
                    if requested > unlocked as u16 { return Ok(Ok(None)); }
                    let state = sqlx::query("SELECT character_revision::text,level,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                    if numeric_u64(&state,"character_revision")? != root.revision || !state_matches_root(&state,&root) { return Err(DurabilityError::InvalidStoredState); }
                    let changed = requested != current as u16;
                    let next = root.revision.checked_add(u64::from(changed)).ok_or(DurabilityError::InvalidStoredState)?;
                    selected = requested;
                    response = vec![unlocked as u8, requested as u8];
                    sqlx::query("INSERT INTO game_character_hazard_receipts(character_id,command_session_id,connection_generation,command_id,selected_requested,result,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) SELECT s.character_id,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5,$6,s.character_revision,$7::text::numeric,s.level,s.level,s.total_experience,s.total_experience,s.profile_revision,s.ruleset_revision,s.content_revision,s.simulation_revision,s.evidence_revision,s.declaration_revision,s.policy_revision,s.reward_revision FROM game_character_progression_state s WHERE s.character_id=encode($1,'hex')::uuid")
                        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice())
                        .bind(fence.connection_generation.get().to_string()).bind(id.to_string()).bind(requested as i16).bind(&response)
                        .bind(changed.then(|| next.to_string())).execute(&mut *tx).await?;
                    if changed {
                        for query in [
                            "UPDATE game_character_roots SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric",
                            "UPDATE game_character_progression_state SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric",
                        ] {
                            if sqlx::query(query).bind(fence.character_id.as_bytes().as_slice()).bind(next.to_string()).bind(root.revision.to_string()).execute(&mut *tx).await?.rows_affected() != 1 { return Err(DurabilityError::InvalidStoredState); }
                        }
                        sqlx::query("UPDATE game_character_hazard SET selected=$2 WHERE character_id=encode($1,'hex')::uuid")
                            .bind(fence.character_id.as_bytes().as_slice()).bind(requested as i16).execute(&mut *tx).await?;
                    }
                }
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(Some((unlocked as u16, selected, response))))
        })).await?
    }
}
