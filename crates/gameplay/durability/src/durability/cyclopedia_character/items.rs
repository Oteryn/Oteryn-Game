//! Self custody in one fenced, read-only snapshot, including closed nested bags.
use super::*;
use oteryn_protocol_oteryn::cyclopedia_character::items::{Page, Row as ItemRow};
pub const HOLDINGS_SQL: &str = r#"
WITH RECURSIVE owner AS (
 SELECT character_id,world_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND lifecycle=1
), roots AS (
 SELECT s.item_instance_id,s.world_id,0 kind FROM owner JOIN game_item_container_slots s USING(character_id,world_id)
 UNION ALL SELECT s.item_instance_id,s.world_id,0 FROM owner JOIN game_character_equipment_slots s USING(character_id,world_id)
 UNION ALL SELECT s.item_instance_id,s.world_id,1 FROM owner JOIN game_item_character_depot_locations s USING(character_id,world_id)
 UNION ALL SELECT s.item_instance_id,s.world_id,3 FROM owner JOIN game_item_character_inbox_locations s USING(character_id,world_id)
), edges AS (
 SELECT item_instance_id,world_id,parent_item_instance_id FROM game_item_container_entries
 UNION ALL SELECT item_instance_id,world_id,parent_item_instance_id FROM game_item_nested_entries
), tree AS (
 SELECT * FROM roots
 UNION SELECT e.item_instance_id,e.world_id,t.kind FROM tree t JOIN edges e ON e.parent_item_instance_id=t.item_instance_id AND e.world_id=t.world_id
), units AS (
 SELECT i.definition_production_key key,t.kind,i.quantity::bigint quantity FROM tree t JOIN game_item_instances i USING(item_instance_id,world_id) WHERE i.lifecycle=1 AND i.definition_family='Item'
 UNION ALL SELECT b.definition_key,2,b.quantity FROM owner JOIN game_character_stash_balances b USING(character_id,world_id) WHERE b.quantity>0
), grouped AS (
 SELECT key COLLATE "C" key,
 COALESCE(sum(quantity) FILTER(WHERE kind=0),0)::bigint inventory,
 COALESCE(sum(quantity) FILTER(WHERE kind=1),0)::bigint depot,
 COALESCE(sum(quantity) FILTER(WHERE kind=2),0)::bigint stash,
 COALESCE(sum(quantity) FILTER(WHERE kind=3),0)::bigint inbox
 FROM units GROUP BY key COLLATE "C"
)
SELECT page.*,totals.total FROM (SELECT count(*)::bigint total FROM grouped) totals
LEFT JOIN (SELECT * FROM grouped ORDER BY key COLLATE "C" OFFSET $2 LIMIT 24) page ON true
ORDER BY page.key COLLATE "C"
"#;
impl DurabilityRoot {
    pub async fn read_cyclopedia_item_summary(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        offset: u16,
    ) -> Result<Page, CharacterAuthorityError> {
        let recovery = authority.record_for(self)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if super::super::character_progression::assert_item_only_fence(
                        &mut tx, &fence, &node,
                    )
                    .await?
                    .is_err()
                    {
                        return Ok(Err(CharacterAuthorityError::Rejected));
                    }
                    let records = sqlx::query(HOLDINGS_SQL)
                        .bind(fence.character_id.as_bytes().as_slice())
                        .bind(i64::from(offset))
                        .fetch_all(&mut *tx)
                        .await?;
                    let total = records
                        .first()
                        .map(|r| r.try_get::<i64, _>("total"))
                        .transpose()?
                        .unwrap_or(0);
                    let mut rows = Vec::with_capacity(records.len());
                    for record in records {
                        let Some(definition) = record.try_get::<Option<String>, _>("key")? else {
                            continue;
                        };
                        rows.push(ItemRow {
                            definition,
                            quantities: [
                                narrow(record.try_get("inventory")?)?,
                                narrow(record.try_get("depot")?)?,
                                narrow(record.try_get("stash")?)?,
                                narrow(record.try_get("inbox")?)?,
                            ],
                        });
                    }
                    let page = Page {
                        offset,
                        total: narrow(total)?,
                        rows,
                    };
                    if page.encode().is_err() {
                        return Ok(Err(CharacterAuthorityError::Rejected));
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(page))
                })
            })
            .await?
    }
}

#[cfg(test)]
mod tests {
    use crate::bestiary_postgres_harness as harness;
    use harness::{Harness, TestResult, debug, id};

    #[test]
    fn holdings_boundary_page_preserves_total_postgres() -> TestResult {
        let Some(admin) = harness::configured_admin() else {
            return Ok(());
        };
        std::thread::Builder::new().stack_size(64 << 20).spawn(move || {
            harness::runtime().map_err(|error| error.to_string())?.block_on(async move {
                let h = Harness::create(admin, "holdings_boundary", true).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                let fence = harness::item_fence()?;
                let empty = h.root.read_cyclopedia_item_summary(&authority, &h.node, fence, 0)
                    .await.map_err(debug)?;
                assert_eq!((empty.total, empty.rows.len()), (0, 0));
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                for index in 0..24_u8 {
                    sqlx::query("INSERT INTO game_character_stash_balances(character_id,world_id,definition_key,definition_revision,quantity,last_entry_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,'definition-r1',1,encode($4,'hex')::uuid)")
                        .bind(fence.character_id.as_bytes().as_slice())
                        .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                        .bind(format!("oteryn:item.test.{index:02}"))
                        .bind(id(100 + index).as_slice()).execute(&mut *tx).await?;
                }
                tx.commit().await?;
                let first = h.root.read_cyclopedia_item_summary(&authority, &h.node, fence, 0)
                    .await.map_err(debug)?;
                assert_eq!((first.total, first.rows.len()), (24, 24));
                assert!(first.rows.iter().all(|row| row.quantities == [0, 0, 1, 0]));
                let boundary = h.root.read_cyclopedia_item_summary(&authority, &h.node, fence, 24)
                    .await.map_err(debug)?;
                assert_eq!((boundary.offset, boundary.total, boundary.rows.len()), (24, 24, 0));
                assert_eq!(super::Page::decode(&boundary.encode().map_err(debug)?).map_err(debug)?, boundary);
                assert!(h.root.read_cyclopedia_item_summary(&authority, &h.node, fence, 25).await.is_err());
                drop(authority);
                drop(seal);
                h.cleanup().await
            }).map_err(|error: Box<dyn std::error::Error>| error.to_string())
        })?.join().map_err(|_| "holdings regression thread failed")??;
        Ok(())
    }
}
