//! Logout snapshots are written only under the current native character/session fence.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_transfer::CurrentCharacterItemFence;
use super::item_use::ItemUseError;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::durability::db::commit_semantic_read;
use sqlx::Row;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct CharacterConditionSnapshot {
    pub lease_generation: u64,
    pub content_digest: [u8; 32],
    pub conditions: serde_json::Value,
    /// A live PvE logout deadline on this process's clock. The database
    /// persists the time remaining against its own `saved_at` clock.
    pub pve_logout_deadline: Option<Instant>,
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
            let sampled_at = Instant::now();
            let row = sqlx::query("SELECT character_lease_generation::text, content_digest, \
                snapshot - 'pve_logout_remaining_us' AS snapshot, \
                GREATEST(0, COALESCE((snapshot->>'pve_logout_remaining_us')::bigint, 0) \
                    - GREATEST(0, (extract(epoch FROM (clock_timestamp() - saved_at)) * 1000000)::bigint)) AS pve_logout_remaining_us \
                FROM game_character_condition_snapshots WHERE character_id=encode($1,'hex')::uuid")
                .bind(character.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let saved = row.map(|row| {
                let remaining = row.try_get::<i64, _>("pve_logout_remaining_us")?;
                Ok::<_, DurabilityError>(CharacterConditionSnapshot {
                    lease_generation: row.try_get::<String, _>("character_lease_generation")?.parse().map_err(|_| DurabilityError::InvalidStoredState)?,
                    content_digest: row.try_get::<Vec<u8>, _>("content_digest")?.try_into().map_err(|_| DurabilityError::InvalidStoredState)?,
                    conditions: row.try_get("snapshot")?,
                    pve_logout_deadline: if remaining > 0 {
                        Some(sampled_at.checked_add(Duration::from_micros(remaining as u64))
                            .ok_or(DurabilityError::InvalidStoredState)?)
                    } else { None },
                })
            }).transpose()?;
            commit_semantic_read(tx, deadline).await?;
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
            // Keep the optional wall-time metadata outside the condition codec. A
            // repeated checkout uses the original cutoff and saves less time.
            let mut snapshot = saved.conditions;
            if let Some(deadline) = saved.pve_logout_deadline {
                let remaining = deadline.saturating_duration_since(Instant::now());
                let micros = i64::try_from(remaining.as_micros()).map_err(|_| DurabilityError::InvalidStoredState)?;
                if micros > 0 {
                    snapshot["pve_logout_remaining_us"] = serde_json::Value::from(micros);
                }
            }
            sqlx::query("INSERT INTO game_character_condition_snapshots(character_id,character_lease_generation,content_digest,snapshot) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4) ON CONFLICT(character_id) DO UPDATE SET character_lease_generation=EXCLUDED.character_lease_generation,content_digest=EXCLUDED.content_digest,snapshot=EXCLUDED.snapshot,saved_at=EXCLUDED.saved_at")
                .bind(fence.character_id.as_bytes().as_slice())
                .bind(saved.lease_generation.to_string())
                .bind(saved.content_digest.as_slice()).bind(snapshot).execute(&mut *tx).await?;
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
            pve_logout_deadline: None,
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
