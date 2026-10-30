use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{CurrentCharacterGameplayFence, assert_gameplay_fence};
use super::db::{begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use oteryn_protocol_oteryn::cyclopedia_tracker::TrackerAction;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackerKind { Bestiary = 1, Bosstiary = 2 }

impl DurabilityRoot {
    pub async fn update_cyclopedia_tracker(&self, authority: &ReconciledCharacterAuthority<'_, '_>, node: &NodeIncarnationProof, fence: CurrentCharacterGameplayFence, kind: TrackerKind, action: TrackerAction, creature_key: &str) -> Result<Vec<String>, DurabilityError> {
        if creature_key.is_empty() || creature_key.len() > 128 { return Err(DurabilityError::InvalidStoredState); }
        let recovery = authority.record_for(self).map_err(|_| DurabilityError::InvalidStoredState)?;
        let node = node.clone(); let key = creature_key.to_owned();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if assert_gameplay_fence(&mut tx, &fence, &node).await?.is_err() { return Err(DurabilityError::InvalidStoredState); }
            if kind == TrackerKind::Bosstiary && action == TrackerAction::Add {
                let killed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_bosstiary_progress WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2 AND kill_count>0)")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(&key).fetch_one(&mut *tx).await?;
                if !killed { return Err(DurabilityError::InvalidStoredState); }
            }
            match action {
                TrackerAction::Add => { sqlx::query("INSERT INTO game_character_cyclopedia_tracker(character_id,tracker_kind,creature_key) VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING").bind(fence.character_id.as_bytes().as_slice()).bind(kind as i16).bind(&key).execute(&mut *tx).await?; }
                TrackerAction::Remove => { sqlx::query("DELETE FROM game_character_cyclopedia_tracker WHERE character_id=encode($1,'hex')::uuid AND tracker_kind=$2 AND creature_key=$3").bind(fence.character_id.as_bytes().as_slice()).bind(kind as i16).bind(&key).execute(&mut *tx).await?; }
            }
            let keys = sqlx::query_scalar("SELECT creature_key FROM game_character_cyclopedia_tracker WHERE character_id=encode($1,'hex')::uuid AND tracker_kind=$2 ORDER BY creature_key LIMIT 256")
                .bind(fence.character_id.as_bytes().as_slice()).bind(kind as i16).fetch_all(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?; Ok(keys)
        })).await
    }
}
