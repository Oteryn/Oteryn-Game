//! Crystal 201189f player.cpp:5304-5356,5821-5844: carried DFS, then stash.
use super::*;

pub(super) struct MaterialPlan {
    pub(super) inputs: Vec<Value>,
    pub(super) stash: Option<Value>,
    pub(super) equipment: Option<Value>,
    pub(super) burned_weight: u64,
}

pub(super) async fn plan(
    tx: &mut Type2Transaction<'_>,
    fence: &CurrentCharacterGameplayFence,
    request: &AtelierRequest<'_>,
    resources: &AtelierResources<'_>,
    key: &str,
    mut needed: u32,
) -> std::result::Result<Option<MaterialPlan>, DurabilityError> {
    if !(1..=30).contains(&needed) { return Ok(None); }
    // Root -> equipment -> item UUIDs -> stash is also the normal custody lock order.
    let equipment: Option<Value> = sqlx::query_scalar("SELECT to_jsonb(s) FROM game_character_equipment_state s WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
    // Durable equipment codes map to Crystal's slot order (item_view.rs:210-224).
    // Lexicographic ordinal paths implement ContainerIterator's DFS, not breadth-first.
    let rows = sqlx::query(
        "WITH RECURSIVE roots AS (
           SELECT b.item_instance_id,3 AS slot_order,0::smallint AS slot,'backpack'::text AS location
             FROM game_item_container_slots b WHERE b.character_id=encode($1,'hex')::uuid AND b.world_id=encode($2,'hex')::uuid
           UNION ALL
           SELECT s.item_instance_id,CASE s.slot WHEN 1 THEN 1 WHEN 7 THEN 2 WHEN 2 THEN 4 WHEN 5 THEN 5 WHEN 6 THEN 6 WHEN 3 THEN 7 WHEN 4 THEN 8 WHEN 8 THEN 9 ELSE 10 END,s.slot,'equipment'
             FROM game_character_equipment_slots s WHERE s.character_id=encode($1,'hex')::uuid AND s.world_id=encode($2,'hex')::uuid
         ), carried AS (
           SELECT r.item_instance_id,NULL::uuid AS parent_item_instance_id,0::numeric AS placement_ordinal,r.slot_order,r.slot,r.location,0 AS depth,ARRAY[]::numeric[] AS path,ARRAY[r.item_instance_id] AS ancestors
             FROM roots r JOIN game_item_instances i USING(item_instance_id) WHERE i.lifecycle=1 AND i.world_id=encode($2,'hex')::uuid
           UNION ALL
           SELECT e.item_instance_id,e.parent_item_instance_id,e.placement_ordinal,c.slot_order,c.slot,
                  CASE WHEN c.location='backpack' THEN 'main' ELSE 'nested' END,c.depth+1,c.path||e.placement_ordinal,c.ancestors||e.item_instance_id
             FROM carried c JOIN (
               SELECT item_instance_id,parent_item_instance_id,placement_ordinal,world_id FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid
               UNION ALL SELECT item_instance_id,parent_item_instance_id,placement_ordinal,world_id FROM game_item_nested_entries
             ) e ON e.parent_item_instance_id=c.item_instance_id AND e.world_id=encode($2,'hex')::uuid
             JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id AND i.world_id=e.world_id AND i.lifecycle=1
            WHERE c.depth<9 AND NOT e.item_instance_id=ANY(c.ancestors)
         ), bounded AS MATERIALIZED (
           SELECT * FROM carried LIMIT 510
         ), locked AS MATERIALIZED (
           SELECT i.item_instance_id FROM game_item_instances i JOIN bounded c USING(item_instance_id) ORDER BY i.item_instance_id FOR UPDATE OF i
         )
         SELECT to_jsonb(i) AS before,i.item_instance_id::text,i.quantity,i.definition_family,i.definition_production_key,i.definition_revision_ref,
                c.parent_item_instance_id::text,c.placement_ordinal::text,c.location,c.slot,c.depth,
                (NOT EXISTS(SELECT 1 FROM game_item_keys k WHERE k.item_instance_id=i.item_instance_id)
                 AND NOT EXISTS(SELECT 1 FROM game_item_timed_states t WHERE t.item_instance_id=i.item_instance_id)
                 AND NOT EXISTS(SELECT 1 FROM game_item_texts t WHERE t.item_instance_id=i.item_instance_id)
                 AND NOT EXISTS(SELECT 1 FROM game_item_imbuements b WHERE b.item_instance_id=i.item_instance_id)
                 AND NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.parent_item_instance_id=i.item_instance_id)
                 AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries e WHERE e.parent_item_instance_id=i.item_instance_id)) AS plain
           FROM bounded c JOIN locked l USING(item_instance_id) JOIN game_item_instances i USING(item_instance_id)
          ORDER BY c.slot_order,c.path DESC",
    ).bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).fetch_all(&mut ***tx).await?;
    if rows.len() >= 510 { return Ok(None); }
    let mut seen = std::collections::BTreeSet::new();
    let mut tree_items = 0_usize;
    let mut equipment_roots = 0_usize;
    for row in &rows {
        if row.try_get::<i32, _>("depth")? > 8 || !seen.insert(row.try_get::<String, _>("item_instance_id")?) { return Ok(None); }
        let equipped = row.try_get::<String, _>("location")? == "equipment";
        tree_items += usize::from(!equipped);
        equipment_roots += usize::from(equipped);
    }
    if tree_items > 500 || equipment_roots > 9 { return Ok(None); }
    let mut result = MaterialPlan { inputs: Vec::new(), stash: None, equipment: None, burned_weight: 0 };
    for row in rows {
        if needed == 0 { break; }
        let definition = TypedDefinitionRef { family: row.try_get("definition_family")?, production_key: row.try_get("definition_production_key")?, revision_ref: row.try_get("definition_revision_ref")? };
        let quantity: i64 = row.try_get("quantity")?;
        if definition.family != "Item" || definition.production_key != key || !(1..=100).contains(&quantity) || !row.try_get::<bool, _>("plain")? { continue; }
        let Some(facts) = resources.content.loop_facts(&definition) else { return Ok(None); };
        let Some(weight) = facts.weight else { return Ok(None); };
        if super::super::super::item_transfer::stack_maximum(facts.stack).map(|(n, _)| n) != Ok(100) { return Ok(None); }
        let quantity = u32::try_from(quantity).map_err(|_| DurabilityError::InvalidStoredState)?;
        let count = needed.min(quantity);
        needed -= count;
        result.burned_weight += u64::from(weight) * u64::from(count);
        let location: String = row.try_get("location")?;
        if location == "equipment" {
            let Some(before) = &equipment else { return Err(DurabilityError::InvalidStoredState); };
            result.equipment = Some(before.clone());
        }
        result.inputs.push(json!({"item":row.try_get::<String,_>("item_instance_id")?,"ordinal":row.try_get::<String,_>("placement_ordinal")?,"parent":row.try_get::<Option<String>,_>("parent_item_instance_id")?,"location":location,"slot":row.try_get::<i16,_>("slot")?,"before":row.try_get::<Value,_>("before")?,"count":count}));
    }
    if needed == 0 { return Ok(Some(result)); }
    if !matches!(request.mutation, AtelierMutation::GradeUp { .. }) || resources.fragment.family != "Item" || resources.fragment.production_key != key { return Ok(None); }
    // Stash remains in its existing Character/World/revision balance and ledger. Reveal
    // deliberately never reaches this fallback (player_wheel.cpp:1233 passes false).
    let balance = sqlx::query("SELECT quantity,last_entry_id::text FROM game_character_stash_balances WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND definition_key=$3 AND definition_revision=$4 FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(key).bind(&resources.fragment.revision_ref).fetch_optional(&mut ***tx).await?;
    let Some(balance) = balance else { return Ok(None); };
    let before: i64 = balance.try_get("quantity")?;
    if before < i64::from(needed) { return Ok(None); }
    let entry: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text").fetch_one(&mut ***tx).await?;
    result.stash = Some(json!({"entry":entry,"key":key,"revision":resources.fragment.revision_ref,"count":needed,"quantity_before":before,"previous":balance.try_get::<String,_>("last_entry_id")?}));
    Ok(Some(result))
}

pub(super) async fn apply_stash(
    tx: &mut Type2Transaction<'_>,
    fence: &CurrentCharacterGameplayFence,
    transaction: &str,
    stash: &Value,
) -> std::result::Result<(), DurabilityError> {
    sqlx::query("INSERT INTO game_character_stash_entries(entry_id,atelier_transaction_id,character_id,world_id,definition_key,definition_revision,amount,quantity_before,quantity_after,previous_entry_id) VALUES(($1->>'entry')::uuid,$2::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$1->>'key',$1->>'revision',($1->>'count')::bigint,($1->>'quantity_before')::bigint,($1->>'quantity_before')::bigint-($1->>'count')::bigint,($1->>'previous')::uuid)")
        .bind(stash).bind(transaction).bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).execute(&mut ***tx).await?;
    let changed = sqlx::query("UPDATE game_character_stash_balances SET quantity=quantity-($1->>'count')::bigint,last_entry_id=($1->>'entry')::uuid WHERE character_id=encode($2,'hex')::uuid AND world_id=encode($3,'hex')::uuid AND definition_key=$1->>'key' AND definition_revision=$1->>'revision' AND quantity=($1->>'quantity_before')::bigint AND last_entry_id=($1->>'previous')::uuid")
        .bind(stash).bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).execute(&mut ***tx).await?;
    if changed.rows_affected() != 1 { return Err(DurabilityError::InvalidStoredState); }
    Ok(())
}
