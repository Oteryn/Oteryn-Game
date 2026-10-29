//! Read side of depot search; existing depot and stash owners retain all write authority.
use super::{DurabilityError, DurabilityRoot};
use crate::domain::CharacterId;
use oteryn_protocol_oteryn::depot_search::{SearchResult, TierCount};
use oteryn_protocol_oteryn::stash::StashRow;
use sqlx::Row;

impl DurabilityRoot {
    /// Resolve one deterministic stash revision for a depot-search retrieval.
    /// The returned handle is server-issued and is still revalidated and locked by
    /// `stash_conversion`; this read never grants mutation authority by itself.
    pub async fn read_stash_definition(
        &self,
        recovery: &super::character_authority::ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        definition_key: &str,
    ) -> Result<Option<StashRow>, DurabilityError> {
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let definition_key = definition_key.to_owned();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
            super::character_authority::assert_recovery_fence(&mut tx, &record).await?;
            let row = sqlx::query("SELECT uuid_send(b.last_entry_id) AS handle,b.definition_key,b.definition_revision,b.quantity FROM game_character_stash_balances b JOIN game_character_roots r USING(character_id,world_id) WHERE b.character_id=encode($1,'hex')::uuid AND b.definition_key=$2 AND b.quantity>0 AND r.lifecycle=1 ORDER BY b.definition_revision COLLATE \"C\" LIMIT 1")
                .bind(character.as_bytes().as_slice()).bind(definition_key).fetch_optional(&mut *tx).await?;
            let row = row.map(|row| -> Result<StashRow, DurabilityError> {
                Ok(StashRow {
                    handle: row.try_get::<Vec<u8>, _>("handle")?.try_into().map_err(|_| DurabilityError::InvalidStoredState)?,
                    definition_key: row.try_get("definition_key")?,
                    definition_revision: row.try_get("definition_revision")?,
                    quantity: u32::try_from(row.try_get::<i64, _>("quantity")?).map_err(|_| DurabilityError::InvalidStoredState)?,
                    migration_required: false,
                })
            }).transpose()?;
            super::db::commit_semantic_transaction(tx, deadline).await?;
            Ok(row)
        })).await
    }

    /// One snapshot of the live Character's World-scoped custody aggregates.
    pub async fn read_depot_search(
        &self,
        recovery: &super::character_authority::ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        client_id: u32,
    ) -> Result<Option<SearchResult>, DurabilityError> {
        if client_id == 0 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
            super::character_authority::assert_recovery_fence(&mut tx, &record).await?;
            let key = format!("oteryn:item.tibia.i{client_id}");
            // Custody families hold whole, content-free items. The root binds the World;
            // no Channel filtering may hide possessions after a Channel change. Aggregation
            // returns one row and at most eleven tier counters, never item identities.
            let row = sqlx::query(r#"
                WITH owner AS (
                    SELECT character_id, world_id FROM game_character_roots
                    WHERE character_id=encode($1,'hex')::uuid AND lifecycle=1
                ), held AS (
                    SELECT 'depot' kind, i.quantity, COALESCE(t.tier,0) tier
                    FROM owner o JOIN game_item_character_depot_locations l USING(character_id,world_id)
                    JOIN game_item_instances i USING(item_instance_id,world_id)
                    LEFT JOIN game_item_tiers t USING(item_instance_id)
                    WHERE i.definition_family='Item' AND i.definition_production_key=$2 AND i.lifecycle=1
                    UNION ALL
                    SELECT 'inbox', i.quantity, COALESCE(t.tier,0)
                    FROM owner o JOIN game_item_character_inbox_locations l USING(character_id,world_id)
                    JOIN game_item_instances i USING(item_instance_id,world_id)
                    LEFT JOIN game_item_tiers t USING(item_instance_id)
                    WHERE i.definition_family='Item' AND i.definition_production_key=$2 AND i.lifecycle=1
                ), counts AS (
                    SELECT COALESCE(sum(quantity) FILTER(WHERE kind='depot'),0)::bigint depot,
                           COALESCE(sum(quantity) FILTER(WHERE kind='inbox'),0)::bigint inbox FROM held
                ), stash AS (
                    SELECT COALESCE(sum(b.quantity),0)::bigint stash
                    FROM owner o JOIN game_character_stash_balances b USING(character_id,world_id)
                    WHERE b.definition_key=$2
                ), tiers AS (
                    SELECT COALESCE(jsonb_agg(jsonb_build_array(tier,total) ORDER BY tier),'[]') value
                    FROM (SELECT tier, sum(quantity)::bigint total FROM (
                        SELECT tier,quantity FROM held
                        UNION ALL SELECT 0,stash FROM stash WHERE stash>0
                    ) units GROUP BY tier) q
                )
                SELECT depot,inbox,stash,value::text tiers FROM counts,stash,tiers
            "#)
                .bind(character.as_bytes().as_slice()).bind(&key)
                .fetch_one(&mut *tx).await?;
            let count = |name| -> Result<u32, DurabilityError> {
                u32::try_from(row.try_get::<i64,_>(name)?).map_err(|_| DurabilityError::InvalidStoredState)
            };
            let (depot, inbox, stash) = (count("depot")?, count("inbox")?, count("stash")?);
            let raw: Vec<(u8,u32)> = serde_json::from_str(&row.try_get::<String,_>("tiers")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            let tiers = raw.into_iter().map(|(tier,count)| TierCount {tier,count}).collect();
            super::db::commit_semantic_transaction(tx,deadline).await?;
            Ok((depot!=0 || inbox!=0 || stash!=0).then_some(SearchResult {client_id,depot,stash,inbox,tiers}))
        })).await
    }
}
