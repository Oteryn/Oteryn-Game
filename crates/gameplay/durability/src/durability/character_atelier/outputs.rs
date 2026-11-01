//! Crystal 201189f player.cpp:4932-5047: ordinary carried destinations, breadth first.
//! The Character fence owns the graph; current item rows are locked before planning.
use super::*;
use std::collections::BTreeSet;

struct CarriedItem {
    id: String,
    parent: Option<String>,
    ordinal: u64,
    surface: i16,
    depth: i32,
    definition: TypedDefinitionRef,
    quantity: u32,
    before: Value,
    plain: bool,
}

pub(super) async fn plan(
    tx: &mut Type2Transaction<'_>,
    fence: &CurrentCharacterGameplayFence,
    resources: &AtelierResources<'_>,
    key: &str,
    count: u32,
    burned_weight: u64,
) -> std::result::Result<Option<Vec<Value>>, DurabilityError> {
    if resources.fragment.family != "Item"
        || resources.fragment.production_key != key
        || !(1..=10).contains(&count)
    {
        return Ok(None);
    }
    let Some(facts) = resources.content.loop_facts(&resources.fragment) else {
        return Ok(None);
    };
    let Some(weight) = facts.weight else {
        return Ok(None);
    };
    if super::super::super::item_transfer::stack_maximum(facts.stack).map(|(n, _)| n)
        != Ok(100)
    {
        return Ok(None);
    }
    let carried = match carried_weight(
        tx,
        fence.character_id,
        *fence.runtime_scope.world_id().as_bytes(),
        resources.content,
    )
    .await
    {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let limit = super::super::super::item_transfer::read_derived_transfer_capacity_centi_oz(
        tx,
        fence.character_id,
        resources.content,
    )
    .await?;
    if carried
        .checked_sub(burned_weight)
        .and_then(|value| value.checked_add(u64::from(weight) * u64::from(count)))
        .is_none_or(|value| value > limit)
    {
        return Ok(None);
    }
    let rows = sqlx::query(
        "WITH RECURSIVE roots AS (
           SELECT s.item_instance_id,0::smallint AS surface,3 AS root_order
             FROM game_item_container_slots s WHERE s.character_id=encode($1,'hex')::uuid AND s.world_id=encode($2,'hex')::uuid
           UNION ALL
           SELECT s.item_instance_id,3::smallint,CASE s.slot WHEN 1 THEN 1 WHEN 7 THEN 2 WHEN 2 THEN 4 WHEN 5 THEN 5 WHEN 6 THEN 6 WHEN 3 THEN 7 WHEN 4 THEN 8 WHEN 8 THEN 9 WHEN 10 THEN 10 END
             FROM game_character_equipment_slots s WHERE s.character_id=encode($1,'hex')::uuid AND s.world_id=encode($2,'hex')::uuid),
         carried AS (
           SELECT r.item_instance_id,NULL::uuid AS parent_item_instance_id,0::numeric AS placement_ordinal,r.surface,r.root_order,0 AS depth,ARRAY[]::numeric[] AS ordinal_path,ARRAY[r.item_instance_id] AS seen
             FROM roots r JOIN game_item_instances i USING(item_instance_id) WHERE i.lifecycle=1 AND i.world_id=encode($2,'hex')::uuid
           UNION ALL
           SELECT e.item_instance_id,e.parent_item_instance_id,e.placement_ordinal,(CASE WHEN p.surface=0 THEN 1 ELSE 2 END)::smallint,p.root_order,p.depth+1,p.ordinal_path||e.placement_ordinal,p.seen||e.item_instance_id
             FROM carried p JOIN LATERAL (
               SELECT item_instance_id,parent_item_instance_id,placement_ordinal FROM game_item_container_entries
                WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND parent_item_instance_id=p.item_instance_id
               UNION ALL SELECT item_instance_id,parent_item_instance_id,placement_ordinal FROM game_item_nested_entries
                WHERE world_id=encode($2,'hex')::uuid AND parent_item_instance_id=p.item_instance_id) e ON true
             JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id AND i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1
             WHERE p.depth<9 AND NOT e.item_instance_id=ANY(p.seen)),
         bounded AS MATERIALIZED (SELECT * FROM carried LIMIT 510),
         locked AS MATERIALIZED (SELECT i.item_instance_id FROM game_item_instances i JOIN bounded b USING(item_instance_id) ORDER BY i.item_instance_id FOR UPDATE OF i)
         SELECT b.item_instance_id::text,b.parent_item_instance_id::text,b.placement_ordinal::text,b.surface,b.depth,
           i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,to_jsonb(i) AS before,
           (NOT EXISTS(SELECT 1 FROM game_item_keys k WHERE k.item_instance_id=i.item_instance_id)
            AND NOT EXISTS(SELECT 1 FROM game_item_timed_states t WHERE t.item_instance_id=i.item_instance_id)
            AND NOT EXISTS(SELECT 1 FROM game_item_texts t WHERE t.item_instance_id=i.item_instance_id)
            AND NOT EXISTS(SELECT 1 FROM game_item_imbuements m WHERE m.item_instance_id=i.item_instance_id)
            AND NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.parent_item_instance_id=i.item_instance_id)
            AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries e WHERE e.parent_item_instance_id=i.item_instance_id)) AS plain
          FROM bounded b JOIN locked l USING(item_instance_id) JOIN game_item_instances i USING(item_instance_id)
          ORDER BY b.depth,b.root_order,b.ordinal_path DESC",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
    .fetch_all(&mut ***tx)
    .await?;
    // Native carried snapshots admit 500 tree rows, including the main bag, and nine
    // separate equipment roots. Keep the overflow sentinel instead of truncating custody.
    if rows.len() >= 510 {
        return Ok(None);
    }
    let mut items = Vec::with_capacity(rows.len());
    let mut seen = BTreeSet::new();
    let mut tree_items = 0_usize;
    let mut equipment_roots = 0_usize;
    for row in rows {
        let id: String = row.try_get("item_instance_id")?;
        let surface: i16 = row.try_get("surface")?;
        let depth: i32 = row.try_get("depth")?;
        if depth > 8 || !seen.insert(id.clone()) {
            return Ok(None);
        }
        tree_items += usize::from(surface != 3);
        equipment_roots += usize::from(surface == 3);
        items.push(CarriedItem {
            id,
            parent: row.try_get("parent_item_instance_id")?,
            ordinal: row.try_get::<String, _>("placement_ordinal")?.parse().map_err(|_| DurabilityError::InvalidStoredState)?,
            surface,
            depth,
            definition: TypedDefinitionRef {
                family: row.try_get("definition_family")?,
                production_key: row.try_get("definition_production_key")?,
                revision_ref: row.try_get("definition_revision_ref")?,
            },
            quantity: u32::try_from(row.try_get::<i64, _>("quantity")?).map_err(|_| DurabilityError::InvalidStoredState)?,
            before: row.try_get("before")?,
            plain: row.try_get("plain")?,
        });
    }
    if tree_items > 500 || equipment_roots > 9 {
        return Ok(None);
    }
    let mut remaining = count;
    let mut outputs = Vec::new();
    // Fragments have no native equipment slot. Containers in equipped slots still take
    // their donor inventory order as roots of the ordinary breadth-first destination walk.
    for container in &items {
        let Some(facts) = resources.content.loop_facts(&container.definition) else {
            return Ok(None);
        };
        let Some(capacity) = facts.container_capacity else {
            continue;
        };
        if resources.content.is_quiver(&container.definition) {
            continue;
        }
        let children: Vec<_> = items.iter().filter(|item| item.parent.as_ref() == Some(&container.id)).collect();
        let capacity = usize::try_from(capacity.min(20)).map_err(|_| DurabilityError::InvalidStoredState)?;
        if children.len() > capacity {
            return Ok(None);
        }
        for receiver in &children {
            if receiver.definition != resources.fragment || !receiver.plain || !(1..100).contains(&receiver.quantity) {
                continue;
            }
            let added = remaining.min(100 - receiver.quantity);
            outputs.push(json!({"item":receiver.id,"parent":container.id,"location":if container.surface==0 { "main" } else { "nested" },"ordinal":receiver.ordinal.to_string(),"key":key,"revision":resources.fragment.revision_ref,"count":added,"before":receiver.before}));
            remaining -= added;
            if remaining == 0 {
                return Ok(Some(outputs));
            }
        }
        if children.len() < capacity && container.depth < 8 && tree_items < 500 {
            let ordinal = children.iter().map(|item| item.ordinal).max().unwrap_or(0).checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
            let item: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text").fetch_one(&mut ***tx).await?;
            outputs.push(json!({"item":item,"parent":container.id,"location":if container.surface==0 { "main" } else { "nested" },"ordinal":ordinal.to_string(),"key":key,"revision":resources.fragment.revision_ref,"count":remaining}));
            return Ok(Some(outputs));
        }
    }
    Ok(None)
}
