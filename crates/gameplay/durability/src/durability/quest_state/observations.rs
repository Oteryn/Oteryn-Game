//! Bounded read-only carried evidence for source-authored NPC voices.
use super::*;

impl DurabilityRoot {
    pub async fn read_quest_carried_counts(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        fence: CurrentCharacterGameplayFence,
        keys: Vec<String>,
    ) -> Result<BTreeMap<String, i64>> {
        if keys.len() > 8
            || keys
                .iter()
                .any(|key| !key.starts_with("oteryn:item.") || !valid_quest_key(key))
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let RuntimeScopeRefV1::Channel { world_id, .. } = fence.runtime_scope else {
            return Err(CharacterProgressionError::AuthorityRejected);
        };
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?.run(move |holder,deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            sqlx::query("SELECT character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR SHARE")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            let rows:Vec<(String,i64)>=sqlx::query_as(
                "WITH RECURSIVE roots AS (
                    SELECT item_instance_id FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid
                    UNION ALL SELECT item_instance_id FROM game_character_equipment_slots WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid
                ), carried(item_instance_id,depth) AS (
                    SELECT item_instance_id,0 FROM roots UNION ALL
                    SELECT e.item_instance_id,c.depth+1 FROM carried c JOIN (
                        SELECT item_instance_id,parent_item_instance_id,world_id FROM game_item_container_entries
                        UNION ALL SELECT item_instance_id,parent_item_instance_id,world_id FROM game_item_nested_entries
                    ) e ON e.parent_item_instance_id=c.item_instance_id AND e.world_id=encode($2,'hex')::uuid WHERE c.depth<8
                ) SELECT i.definition_production_key,sum(i.quantity)::bigint FROM carried c JOIN game_item_instances i USING(item_instance_id)
                    WHERE i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.definition_family='Item'
                        AND i.definition_production_key=ANY($3) GROUP BY i.definition_production_key")
                .bind(fence.character_id.as_bytes().as_slice()).bind(world_id.as_bytes().as_slice()).bind(&keys)
                .fetch_all(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Ok(rows.into_iter().collect()))
        })).await?
    }
}

/// A fresh read-only clock observation; value writes still use the fenced writer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RewardReadiness {
    Ready,
    Cooldown,
    Ineligible,
}

impl DurabilityRoot {
    /// Observe source storage and its cooldown together under the Character root lock.
    pub async fn read_quest_reward_readiness(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        fence: CurrentCharacterGameplayFence,
        catalogue: std::sync::Arc<QuestStateCatalogue>,
        transition_key: String,
    ) -> Result<RewardReadiness> {
        let callback = oteryn_gameplay_quests::quest::reward_callbacks::load()
            .map_err(|_| CharacterProgressionError::InvalidInput)?
            .into_iter().find(|callback| callback.key == transition_key && callback.cooldown.is_some())
            .ok_or(CharacterProgressionError::InvalidInput)?;
        let transition = catalogue.transition(&transition_key)
            .filter(|transition| transition.quest == callback.quest)
            .ok_or(CharacterProgressionError::InvalidInput)?.clone();
        let mut keys = transition.effects.iter().map(|effect| effect.track.clone())
            .chain(callback.requires.iter().map(|required| required.track.clone())).collect::<Vec<_>>();
        keys.sort_unstable(); keys.dedup();
        let recovery = authority.record_for(self).map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?.run(move |holder,deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            sqlx::query("SELECT character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR SHARE")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            let rows=sqlx::query("SELECT track_key,quest_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=ANY($2::text[]) ORDER BY track_key FOR SHARE")
                .bind(fence.character_id.as_bytes().as_slice()).bind(&keys).fetch_all(&mut *tx).await?;
            let mut stored=BTreeMap::new();
            for row in rows {
                let key:String=row.try_get("track_key")?;
                let track=catalogue.track(&key).ok_or(DurabilityError::InvalidStoredState)?;
                if row.try_get::<String,_>("quest_key")? != track.quest { return Err(DurabilityError::InvalidStoredState); }
                stored.insert(key,row.try_get("value")?);
            }
            let now:i64=sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint").fetch_one(&mut *tx).await?;
            let storage_ready=callback.storage.iter().all(|guard| catalogue.track(&guard.track).is_some_and(|track|
                (guard.minimum..=guard.maximum).contains(&stored.get(&guard.track).copied().unwrap_or(track.initial))))
                && callback.requires.iter().all(|required| required.holds(&catalogue,&stored));
            let readiness=if !storage_ready { RewardReadiness::Ineligible } else {
                match catalogue.evaluate(&transition,&stored,now) {
                    Ok(_) => RewardReadiness::Ready,
                    Err(QuestRefusal::StageMismatch) => RewardReadiness::Cooldown,
                    Err(_) => RewardReadiness::Ineligible,
                }
            };
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Ok(readiness))
        })).await?
    }
}
