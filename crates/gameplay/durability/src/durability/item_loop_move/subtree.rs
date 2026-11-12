//! Weight the same bounded logical tree used by SQL admission, including direct
//! entries retained by historical death drops until their accepted pickup.
use super::*;

pub(super) async fn weight<C: ItemLoopContent>(
    tx: &mut Transaction<'_, Postgres>,
    content: &C,
    item: [u8; 16],
    world: [u8; 16],
) -> LoopResult<u64> {
    let rows = sqlx::query(
        "SELECT i.item_instance_id::text AS item,i.definition_family,i.definition_production_key,\
         i.definition_revision_ref,i.quantity,i.lifecycle,i.world_id::text AS world,t.depth,t.cyclic \
         FROM game_item_loop_subtree_edges(encode($1,'hex')::uuid) t \
         JOIN game_item_instances i ON i.item_instance_id=t.item_instance_id \
         ORDER BY i.item_instance_id FOR UPDATE OF i",
    )
    .bind(item.as_slice())
    .fetch_all(&mut **tx)
    .await?;
    // An incoming root also occupies one of the destination's 500 descendant slots.
    if rows.len() > ITEM_LOOP_TREE_ITEMS_MAX as usize {
        return refuse(LoopMoveRefusal::NoRoom);
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut total = 0u64;
    for row in rows {
        if row.try_get::<i16, _>("lifecycle")? != 1
            || uuid_text(row.try_get("world")?)? != world
            || row.try_get::<i32, _>("depth")? > ITEM_LOOP_DEPTH_MAX
            || row.try_get::<bool, _>("cyclic")?
            || !seen.insert(uuid_text(row.try_get("item")?)?)
        {
            return refuse(LoopMoveRefusal::Rejected);
        }
        let quantity = stored_u32(row.try_get("quantity")?)?;
        let unit = match content.weight_of(&definition_of(&row)?) {
            Ok(weight) => weight,
            Err(ItemWeightRefused::Unknown) => return refuse(LoopMoveRefusal::WeightUnknown),
            Err(ItemWeightRefused::DefinitionMismatch) => return refuse(LoopMoveRefusal::Rejected),
        };
        let item_weight = u64::from(unit)
            .checked_mul(u64::from(quantity))
            .ok_or(LoopError::Refused(LoopMoveRefusal::Overweight))?;
        total = total
            .checked_add(item_weight)
            .ok_or(LoopError::Refused(LoopMoveRefusal::Overweight))?;
    }
    if !seen.contains(&item) {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    Ok(total)
}
