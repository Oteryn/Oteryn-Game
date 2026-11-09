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
    let rows = sqlx::query(
        "SELECT i.item_instance_id::text,to_jsonb(i) AS item_before,to_jsonb(e) AS entry_before, \
         (SELECT to_jsonb(f) FROM game_item_fluid_states f WHERE f.item_instance_id=i.item_instance_id FOR UPDATE) AS fluid_before \
         FROM game_item_corpse_container_entries e JOIN game_item_instances i USING(item_instance_id) \
         WHERE e.parent_item_instance_id=encode($1,'hex')::uuid ORDER BY e.placement_ordinal,i.item_instance_id LIMIT $2 FOR UPDATE OF i,e",
    )
    .bind(parent.as_slice())
    .bind(i64::from(crate::durability::item_mint::CORPSE_CONTAINER_ENTRIES_MAX) + 1)
    .fetch_all(&mut ***tx)
    .await?;
    if rows.len() > crate::durability::item_mint::CORPSE_CONTAINER_ENTRIES_MAX as usize {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    for row in rows {
        let child: String = row.try_get("item_instance_id")?;
        let (transaction, event): (String, String) =
            sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text")
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
                "fluid_before":row.try_get::<Option<serde_json::Value>,_>("fluid_before")?, "quantity_after":0,
            }),
            &transaction,
            &event,
            occurred,
            tuple.tuple(),
        )?;
        sqlx::query("SELECT game_elemental_arrival_retire_loot($1::uuid,encode($2,'hex')::uuid,$3::uuid,$4::uuid,$5::uuid,$6,$7,$8,$9)")
            .bind(arrival).bind(parent.as_slice()).bind(child).bind(transaction).bind(event)
            .bind(occurred).bind(envelope).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id())
            .execute(&mut ***tx).await?;
    }
    Ok(())
}
