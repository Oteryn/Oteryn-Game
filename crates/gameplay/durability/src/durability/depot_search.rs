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
        Ok(self
            .read_depot_holdings(recovery, character, Some(client_id), None, 0)
            .await?
            .pop())
    }

    /// Bounded keyset page across all depot, nested container, Inbox and stash holdings.
    /// Detail restricts both custody totals and tier counters to the requested tier.
    pub async fn read_depot_holdings(
        &self,
        recovery: &super::character_authority::ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        client_id: Option<u32>,
        tier: Option<u8>,
        after: u32,
    ) -> Result<Vec<SearchResult>, DurabilityError> {
        if client_id == Some(0) || tier.is_some_and(|tier| tier > 10) {
            return Err(DurabilityError::InvalidStoredState);
        }
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let key = client_id.map(|id| format!("oteryn:item.tibia.i{id}"));
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
            super::character_authority::assert_recovery_fence(&mut tx, &record).await?;
            let rows = sqlx::query(r#"
                WITH RECURSIVE owner AS (
                    SELECT character_id, world_id FROM game_character_roots
                    WHERE character_id=encode($1,'hex')::uuid AND lifecycle=1
                ), roots AS (
                    SELECT l.item_instance_id, l.world_id, 0 kind
                    FROM owner JOIN game_item_character_depot_locations l USING(character_id,world_id)
                    UNION ALL
                    SELECT l.item_instance_id, l.world_id, 2 kind
                    FROM owner JOIN game_item_character_inbox_locations l USING(character_id,world_id)
                    JOIN game_item_instances p USING(item_instance_id,world_id)
                    WHERE p.definition_production_key<>'oteryn:item.tibia.i23721'
                ), tree AS (
                    SELECT * FROM roots
                    UNION
                    SELECT e.item_instance_id, e.world_id, tree.kind
                    FROM tree JOIN game_item_nested_entries e
                      ON e.parent_item_instance_id=tree.item_instance_id AND e.world_id=tree.world_id
                ), units AS (
                    SELECT i.definition_production_key key, COALESCE(t.tier,0)::smallint tier, tree.kind, i.quantity::bigint quantity
                    FROM tree JOIN game_item_instances i USING(item_instance_id,world_id)
                    LEFT JOIN game_item_tiers t USING(item_instance_id)
                    WHERE i.definition_family='Item' AND i.lifecycle=1
                    UNION ALL
                    SELECT b.definition_key, 0::smallint, 1, b.quantity
                    FROM owner JOIN game_character_stash_balances b USING(character_id,world_id)
                    WHERE b.quantity>0
                ), native AS (
                    SELECT substring(key FROM 20)::bigint client_id, tier, kind, quantity
                    FROM units WHERE key ~ '^oteryn:item[.]tibia[.]i[1-9][0-9]{0,9}$'
                      AND ($2::text IS NULL OR key=$2)
                      AND ($3::smallint IS NULL OR tier=$3)
                ), page AS (
                    SELECT DISTINCT client_id FROM native
                    WHERE client_id>$4 AND client_id<=4294967295
                    ORDER BY client_id LIMIT 65
                )
                SELECT client_id,tier,kind,sum(quantity)::bigint quantity
                FROM native JOIN page USING(client_id)
                GROUP BY client_id,tier,kind ORDER BY client_id,tier,kind
            "#)
                .bind(character.as_bytes().as_slice()).bind(key).bind(tier.map(i16::from))
                .bind(i64::from(after)).fetch_all(&mut *tx).await?;
            // 65 ids (one lookahead), eleven tiers, three custody families bound this read.
            let mut units = Vec::with_capacity(rows.len());
            for row in rows {
                units.push((
                    u32::try_from(row.try_get::<i64,_>("client_id")?).map_err(|_| DurabilityError::InvalidStoredState)?,
                    u8::try_from(row.try_get::<i16,_>("tier")?).map_err(|_| DurabilityError::InvalidStoredState)?,
                    row.try_get::<i32,_>("kind")?,
                    u32::try_from(row.try_get::<i64,_>("quantity")?).map_err(|_| DurabilityError::InvalidStoredState)?,
                ));
            }
            let results = aggregate_holdings(units)?;
            super::db::commit_semantic_transaction(tx,deadline).await?;
            Ok(results)
        })).await
    }
}

fn aggregate_holdings(
    units: impl IntoIterator<Item = (u32, u8, i32, u32)>,
) -> Result<Vec<SearchResult>, DurabilityError> {
    let mut results = std::collections::BTreeMap::<u32, SearchResult>::new();
    for (client_id, tier, kind, count) in units {
        if client_id == 0 || tier > 10 || count == 0 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let result = results.entry(client_id).or_insert_with(|| SearchResult {
            client_id,
            depot: 0,
            stash: 0,
            inbox: 0,
            tiers: Vec::new(),
        });
        let total = match kind {
            0 => &mut result.depot,
            1 => &mut result.stash,
            2 => &mut result.inbox,
            _ => return Err(DurabilityError::InvalidStoredState),
        };
        *total = total
            .checked_add(count)
            .ok_or(DurabilityError::InvalidStoredState)?;
        if let Some(existing) = result.tiers.iter_mut().find(|entry| entry.tier == tier) {
            existing.count = existing
                .count
                .checked_add(count)
                .ok_or(DurabilityError::InvalidStoredState)?;
        } else {
            result.tiers.push(TierCount { tier, count });
        }
    }
    for result in results.values_mut() {
        result.tiers.sort_by_key(|entry| entry.tier);
    }
    Ok(results.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depot_list_combines_chests_stash_inbox_stacks_and_tiers() -> Result<(), DurabilityError> {
        let result = aggregate_holdings([
            (3031, 0, 0, 100),
            (3031, 0, 0, 50),
            (3031, 0, 1, 300),
            (3031, 0, 2, 7),
            (3031, 2, 0, 1),
            (100, 0, 2, 1),
        ])?;
        assert_eq!(
            result.iter().map(|r| r.client_id).collect::<Vec<_>>(),
            [100, 3031]
        );
        let row = &result[1];
        assert_eq!((row.depot, row.stash, row.inbox), (151, 300, 7));
        assert_eq!(
            row.tiers,
            [
                TierCount {
                    tier: 0,
                    count: 457
                },
                TierCount { tier: 2, count: 1 }
            ]
        );
        assert!(aggregate_holdings([])?.is_empty());
        assert!(aggregate_holdings([(1, 0, 0, u32::MAX), (1, 0, 0, 1)]).is_err());
        Ok(())
    }
}

#[cfg(test)]
mod postgres_tests {
    use super::*;
    use crate::bestiary_postgres_harness::{self as harness, Harness, TestResult, debug, id};

    #[test]
    fn depot_search_reads_exclude_carried_gold_pouch_tree() -> TestResult {
        let Some(admin) = harness::configured_admin() else { return Ok(()) };
        std::thread::Builder::new().stack_size(64 << 20).spawn(move || {
            harness::runtime().map_err(|error| error.to_string())?.block_on(async move {
                let h = Harness::create(admin, "depot_pouch", true).await?;
                let result = exercise(&h).await;
                h.cleanup().await?;
                result
            }).map_err(|error| error.to_string())
        })?.join().map_err(|_| "depot pouch regression panicked")??;
        Ok(())
    }

    async fn exercise(h: &Harness) -> TestResult {
        // Historical Store provisioning: the pouch is an Inbox root but already carried.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        for (item, definition, quantity) in [(201, 23721, 1), (202, 3031, 100), (203, 3031, 7)] {
            sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',$4,1,encode($1,'hex')::uuid)")
                .bind(id(item).as_slice()).bind(id(harness::WORLD).as_slice()).bind(format!("oteryn:item.tibia.i{definition}")).bind(quantity).execute(&mut *tx).await?;
        }
        for (item, ordinal) in [(201, 1i64), (203, 2)] {
            sqlx::query("INSERT INTO game_item_character_inbox_locations(character_id,world_id,item_instance_id,ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4)")
                .bind(id(harness::CHARACTER).as_slice()).bind(id(harness::WORLD).as_slice()).bind(id(item).as_slice()).bind(ordinal).execute(&mut *tx).await?;
        }
        sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($1,'hex')::uuid)")
            .bind(id(202).as_slice()).bind(id(harness::WORLD).as_slice()).bind(id(201).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let character = CharacterId::from_bytes(id(harness::CHARACTER)).map_err(debug)?;
        let expected = SearchResult { client_id: 3031, depot: 0, stash: 0, inbox: 7, tiers: vec![TierCount { tier: 0, count: 7 }] };
        assert_eq!(h.root.read_depot_search(&authority, character, 3031).await?, Some(expected.clone()));
        assert_eq!(h.root.read_depot_search(&authority, character, 23721).await?, None);
        assert_eq!(h.root.read_depot_holdings(&authority, character, Some(3031), Some(0), 0).await?.as_slice(), std::slice::from_ref(&expected));
        assert_eq!(h.root.read_depot_holdings(&authority, character, None, None, 0).await?, [expected]);
        Ok(())
    }
}
