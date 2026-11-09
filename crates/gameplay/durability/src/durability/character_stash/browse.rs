//! Global view selection happens before LIMIT/OFFSET under the Character read fence.
use super::*;

#[derive(Debug, serde::Serialize)]
pub struct StashCatalogueEntry {
    pub key: String,
    pub revision: String,
    pub name: String,
    pub matched: bool,
}

impl DurabilityRoot {
    pub async fn read_stash_filtered_page(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        character: crate::domain::CharacterId,
        page: u32,
        native: &NativeGameplayState,
        filter: &StashFilter,
        catalogue: &[StashCatalogueEntry],
    ) -> Result<(Vec<StashRow>, bool), DurabilityError> {
        if page > MAX_STASH_PAGE {
            return Err(DurabilityError::InvalidStoredState);
        }
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let metadata =
            serde_json::to_value(catalogue).map_err(|_| DurabilityError::InvalidStoredState)?;
        let restrictive = !filter.search.trim().is_empty()
            || !filter.category.trim().is_empty()
            || !filter.trader.trim().is_empty();
        let stock = filter.stock_order;
        let key_fallback = filter.category.trim().is_empty() && filter.trader.trim().is_empty();
        let search = filter.search.trim().to_lowercase();
        let mut context = native;
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, native| Box::pin(async move {
            let mut tx = super::super::db::begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &record).await?;
            let rows = sqlx::query(r#"
                WITH metadata AS (SELECT * FROM jsonb_to_recordset($3::jsonb) AS m(key text, revision text, name text, matched boolean))
                SELECT uuid_send(b.last_entry_id) AS handle,b.definition_key,b.definition_revision,b.quantity
                FROM game_character_stash_balances b JOIN game_character_roots r USING(character_id,world_id)
                LEFT JOIN metadata m ON m.key=b.definition_key AND m.revision=b.definition_revision
                WHERE b.character_id=encode($1,'hex')::uuid AND b.quantity>0 AND r.lifecycle=1 AND (NOT $4 OR m.matched OR (m.key IS NULL AND $6 AND strpos(lower(b.definition_key),$7)>0))
                ORDER BY CASE WHEN $5 THEN b.quantity END DESC,coalesce(m.name,b.definition_key) COLLATE "C",b.definition_key COLLATE "C",b.definition_revision COLLATE "C"
                LIMIT 33 OFFSET $2
            "#).bind(character.as_bytes().as_slice()).bind(i64::from(page)*32).bind(metadata).bind(restrictive).bind(stock).bind(key_fallback).bind(search).fetch_all(&mut *tx).await?;
            let more = rows.len() > 32;
            let rows = rows.into_iter().take(32).map(|row| {
                let definition = definition(&row)?;
                Ok(StashRow {
                    handle: row.try_get::<Vec<u8>, _>("handle")?.try_into().map_err(|_| DurabilityError::InvalidStoredState)?,
                    migration_required: !stowable(native, &definition),
                    definition_key: definition.production_key, definition_revision: definition.revision_ref,
                    quantity: u32::try_from(row.try_get::<i64,_>("quantity")?).map_err(|_| DurabilityError::InvalidStoredState)?,
                })
            }).collect::<Result<Vec<_>, DurabilityError>>()?;
            super::super::db::commit_semantic_transaction(tx, deadline).await?;
            Ok((rows, more))
        })).await
    }
}
