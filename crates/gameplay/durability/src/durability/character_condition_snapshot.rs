//! Logout snapshots are written only under the current native character/session fence.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_transfer::CurrentCharacterItemFence;
use super::item_use::ItemUseError;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use sqlx::Row;

#[derive(Debug, Clone)]
pub struct CharacterConditionSnapshot {
    pub lease_generation: u64,
    pub content_digest: [u8; 32],
    pub conditions: serde_json::Value,
}
impl CharacterConditionSnapshot {
    fn valid(&self) -> bool {
        self.lease_generation > 0
            && self
                .conditions
                .get("schema")
                .and_then(serde_json::Value::as_u64)
                == Some(1)
            && self
                .conditions
                .get("conditions")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|conditions| conditions.len() <= 32)
            && oteryn_gameplay_foundation::foundation::condition::ConditionStore::from_persistent_snapshot(&self.conditions, 0).is_ok()
    }
}
impl DurabilityRoot {
    /// Data only; the admission owner independently proves its new live authority.
    pub async fn read_character_conditions(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: crate::domain::CharacterId,
    ) -> Result<Option<CharacterConditionSnapshot>, ItemUseError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemUseError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let row = sqlx::query("SELECT character_lease_generation::text,content_digest,snapshot FROM game_character_condition_snapshots WHERE character_id=encode($1,'hex')::uuid")
                .bind(character.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let saved = row.map(|row| {
                Ok::<_, DurabilityError>(CharacterConditionSnapshot {
                    lease_generation: row.try_get::<String, _>("character_lease_generation")?.parse().map_err(|_| DurabilityError::InvalidStoredState)?,
                    content_digest: row.try_get::<Vec<u8>, _>("content_digest")?.try_into().map_err(|_| DurabilityError::InvalidStoredState)?,
                    conditions: row.try_get("snapshot")?,
                })
            }).transpose()?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(saved))
        })).await?
    }

    pub async fn save_character_conditions(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        origin_channel: [u8; 16],
        saved: CharacterConditionSnapshot,
    ) -> Result<(), ItemUseError> {
        if !saved.valid() || saved.lease_generation != fence.character_lease_generation {
            return Err(ItemUseError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemUseError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let current = match fence.runtime_scope {
                crate::foundation::RuntimeScopeRefV1::Instance { .. } => super::house_hireling::fence::assert_house_custody_fence(&mut tx, &node, &fence, origin_channel).await?.is_some(),
                _ => super::house_hireling::fence::assert_inventory_fence(&mut tx, &node, &fence, origin_channel).await?,
            };
            if !current { return Ok(Err(ItemUseError::AuthorityRejected)); }
            sqlx::query("INSERT INTO game_character_condition_snapshots(character_id,character_lease_generation,content_digest,snapshot) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4) ON CONFLICT(character_id) DO UPDATE SET character_lease_generation=EXCLUDED.character_lease_generation,content_digest=EXCLUDED.content_digest,snapshot=EXCLUDED.snapshot,saved_at=clock_timestamp()")
                .bind(fence.character_id.as_bytes().as_slice())
                .bind(saved.lease_generation.to_string())
                .bind(saved.content_digest.as_slice()).bind(saved.conditions).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(()))
        })).await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn character_condition_snapshot_refuses_unversioned_unbounded_or_unfenced_input() {
        let mut snapshot = CharacterConditionSnapshot {
            lease_generation: 1,
            content_digest: [7; 32],
            conditions: serde_json::json!({"schema":1,"conditions":[]}),
        };
        assert!(snapshot.valid());
        snapshot.lease_generation = 0;
        assert!(!snapshot.valid());
        snapshot.lease_generation = 1;
        for conditions in [
            serde_json::Value::Null,
            serde_json::json!({"schema":2,"conditions":[]}),
            serde_json::json!({"schema":1,"conditions":[null]}),
            serde_json::json!({"schema":1,"conditions":vec![serde_json::Value::Null;33]}),
        ] {
            snapshot.conditions = conditions;
            assert!(!snapshot.valid());
        }
    }
}
