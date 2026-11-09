//! Child-first, separately audited logical retirements share the altar's physical commit.
use super::*;

pub(super) async fn retire_contents(
    tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    parent: [u8; 16],
    arrival: &str,
    occurred: i64,
) -> LoopResult<()> {
    // The native atomic kill-MINT writer admits at most 4096 loot items and
    // depth eight. Read one overflow/depth witness and refuse the whole arrival.
    const LOOT_TREE_MAX: usize = 4096;
    let rows = sqlx::query(
        "WITH RECURSIVE tree AS ( \
         SELECT e.item_instance_id,e.parent_item_instance_id,1 AS depth,1 AS entry_kind, \
           ARRAY[e.parent_item_instance_id,e.item_instance_id] AS visited,false AS cycle \
         FROM game_item_corpse_container_entries e WHERE e.parent_item_instance_id=encode($1,'hex')::uuid \
         UNION ALL SELECT e.item_instance_id,e.parent_item_instance_id,t.depth+1,2, \
           t.visited||e.item_instance_id,e.item_instance_id=ANY(t.visited) \
         FROM tree t JOIN game_item_nested_entries e ON e.parent_item_instance_id=t.item_instance_id \
         WHERE t.depth<9 AND NOT t.cycle) \
         SELECT i.item_instance_id::text,uuid_send(t.parent_item_instance_id) AS parent_id,t.depth,t.cycle,t.entry_kind, \
           to_jsonb(i) AS item_before,coalesce(to_jsonb(c),to_jsonb(n)) AS entry_before, \
           (SELECT to_jsonb(f) FROM game_item_fluid_states f WHERE f.item_instance_id=i.item_instance_id FOR UPDATE) AS fluid_before \
         FROM tree t JOIN game_item_instances i USING(item_instance_id) \
         LEFT JOIN game_item_corpse_container_entries c USING(item_instance_id) \
         LEFT JOIN game_item_nested_entries n USING(item_instance_id) \
         ORDER BY t.depth DESC,i.item_instance_id LIMIT $2 FOR UPDATE OF i",
    )
    .bind(parent.as_slice())
    .bind((LOOT_TREE_MAX + 1) as i64)
    .fetch_all(&mut ***tx)
    .await?;
    if rows.len() > LOOT_TREE_MAX
        || rows.iter().any(|row| {
            row.try_get::<i32, _>("depth")
                .map_or(true, |depth| depth > 8)
                || row.try_get::<bool, _>("cycle").unwrap_or(true)
        })
    {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    for row in rows {
        let child: String = row.try_get("item_instance_id")?;
        let (transaction, event): (String, String) =
            sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text")
                .fetch_one(&mut ***tx)
                .await?;
        // Item locks above precede Text-owner revision/live-row locks. The
        // retirement writer locks the exact entry under the held Scope lane.
        // Value audit retains revision metadata only, never player text or a digest.
        let text_state: serde_json::Value =
            sqlx::query_scalar("SELECT game_elemental_loot_text_state($1::uuid)")
                .bind(&child)
                .fetch_one(&mut ***tx)
                .await?;
        let tuple = tx.tuple();
        let envelope = crate::durability::item_craft::audit::encode_map_harvest(
            fence,
            command,
            json!({
                "quest":"oteryn:quest.elemental_spheres_quest", "arrival_transaction":arrival,
                "corpse":parent, "item_before":row.try_get::<serde_json::Value,_>("item_before")?,
                "entry_before":row.try_get::<serde_json::Value,_>("entry_before")?,
                "entry_kind":row.try_get::<i32,_>("entry_kind")?,
                "fluid_before":row.try_get::<Option<serde_json::Value>,_>("fluid_before")?,
                "text_state_before":text_state, "quantity_after":0,
            }),
            &transaction,
            &event,
            occurred,
            tuple.tuple(),
        )?;
        sqlx::query("SELECT game_elemental_arrival_retire_loot($1::uuid,encode($2,'hex')::uuid,$3::uuid,$4::uuid,$5::uuid,$6,$7,$8,$9)")
            .bind(arrival).bind(row.try_get::<Vec<u8>,_>("parent_id")?).bind(child).bind(transaction).bind(event)
            .bind(occurred).bind(envelope).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id())
            .execute(&mut ***tx).await?;
    }
    Ok(())
}
