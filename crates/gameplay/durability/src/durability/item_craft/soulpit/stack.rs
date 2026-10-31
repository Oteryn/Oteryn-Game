//! addItem tries compatible carried stacks before requiring a fresh container slot.
use super::*;

pub(super) async fn find<C: ItemLoopContent>(
    tx: &mut Type2Transaction<'_>,
    context: &CraftContext<'_, C>,
    output: &TypedDefinitionRef,
) -> Result<Option<Input>, DurabilityError> {
    let Some((maximum, true)) = context.content.loop_facts(output).and_then(|facts| stack_maximum(facts.stack).ok()) else { return Ok(None); };
    let row = sqlx::query("SELECT uuid_send(i.item_instance_id) AS item,to_jsonb(i) AS before_item,i.quantity,uuid_send(entry.parent_item_instance_id) AS parent,entry.placement_ordinal::text AS ordinal,entry.nested FROM game_item_instances i JOIN (SELECT item_instance_id,world_id,parent_item_instance_id,placement_ordinal,false AS nested FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid UNION ALL SELECT item_instance_id,world_id,parent_item_instance_id,placement_ordinal,true FROM game_item_nested_entries) entry USING(item_instance_id,world_id) JOIN LATERAL game_item_loop_owner(entry.parent_item_instance_id) owner ON owner.owner_character=encode($1,'hex')::uuid AND owner.depth BETWEEN 0 AND 7 WHERE i.world_id=encode($2,'hex')::uuid AND i.definition_family=$3 AND i.definition_production_key=$4 AND i.definition_revision_ref=$5 AND i.lifecycle=1 AND i.quantity<$6 AND NOT EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id) ORDER BY entry.placement_ordinal DESC,i.item_instance_id LIMIT 1 FOR UPDATE OF i")
        .bind(context.fence.character_id.as_bytes().as_slice()).bind(context.fence.runtime_scope.world_id().as_bytes().as_slice())
        .bind(&output.family).bind(&output.production_key).bind(&output.revision_ref).bind(i64::from(maximum))
        .fetch_optional(&mut ***tx).await?;
    row.map(|row| Ok(Input {
        item: row.try_get::<Vec<u8>,_>("item")?.try_into().map_err(|_| DurabilityError::InvalidStoredState)?,
        before: row.try_get("before_item")?, definition: output.clone(),
        quantity: u32::try_from(row.try_get::<i64,_>("quantity")?).map_err(|_| DurabilityError::InvalidStoredState)?,
        parent: row.try_get::<Vec<u8>,_>("parent")?.try_into().map_err(|_| DurabilityError::InvalidStoredState)?,
        ordinal: row.try_get("ordinal")?, nested: row.try_get("nested")?,
    })).transpose()
}
