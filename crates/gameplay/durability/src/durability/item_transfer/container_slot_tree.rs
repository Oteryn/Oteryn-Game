//! Canonical ContainerSlot admission. Hints grant no custody authority: the
//! complete Item lock set is acquired in one UUID order, then read again.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Tree {
    pub items: Vec<InventoryItem>,
    pub locked: BTreeSet<[u8; 16]>,
}

impl Tree {
    pub fn binding(&self) -> Option<(i16, Vec<u8>)> {
        if self.items.len() == 1 {
            return None;
        }
        let mut ids: Vec<_> = self
            .items
            .iter()
            .map(|item| item.item_instance_id)
            .collect();
        ids.sort_unstable();
        let mut hash = Sha256::new();
        for id in ids {
            hash.update(id);
        }
        Some((self.items.len() as i16, hash.finalize().to_vec()))
    }
}

// BAGS0 counts the root. This separate tree allowance does not change the
// singleton TRANSFER/MINT RL05=0 or their resource/retry/timeout budgets.
fn check_bounds(nodes: usize, height: i32) -> Pass<()> {
    if nodes == 0 || height < 0 {
        return Err(DurabilityError::InvalidStoredState);
    }
    if nodes.checked_add(2).is_none_or(|work| work > 502) || height > 8 {
        // A legal historical death tree can contain 501 Items. Exceeding this
        // promotion allowance is a normal refusal, not corrupt stored state.
        return Ok(Err(ItemTransferError::CapacityExceeded));
    }
    Ok(Ok(()))
}

async fn graph(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    root: [u8; 16],
) -> std::result::Result<Vec<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(sqlx::query(
        "SELECT t.item_instance_id::text,t.world_id::text AS edge_world, \
         t.parent_item_instance_id::text,t.placement_ordinal::text,t.source_kind,t.depth,t.cyclic, \
         to_jsonb(t)::text AS edge_snapshot,to_jsonb(i)::text AS item_snapshot, \
         i.world_id::text AS item_world,i.lifecycle,i.quantity,i.definition_family, \
         i.definition_production_key,i.definition_revision_ref \
         FROM game_item_loop_subtree_edges(encode($1,'hex')::uuid) t \
         LEFT JOIN game_item_instances i USING(item_instance_id) ORDER BY t.item_instance_id",
    )
    .bind(root.as_slice())
    .fetch_all(&mut **tx)
    .await?)
}

fn snapshots(
    rows: &[sqlx::postgres::PgRow],
) -> std::result::Result<Vec<(String, Option<String>)>, DurabilityError> {
    rows.iter()
        .map(|row| Ok((row.try_get("edge_snapshot")?, row.try_get("item_snapshot")?)))
        .collect()
}

// The sentinel LIMIT is inside a materialized CTE, before joining or sorting.
// This bounds both malformed breadth and depth without discarding duplicates.
pub(super) async fn carried_rows(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
) -> std::result::Result<Vec<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(sqlx::query(
        "WITH RECURSIVE roots AS ( \
           SELECT s.item_instance_id,s.world_id,0::smallint AS source_kind, \
                  to_jsonb(s) AS placement FROM game_item_container_slots s \
            WHERE s.character_id=encode($1,'hex')::uuid \
           UNION ALL \
           SELECT s.item_instance_id,s.world_id,3::smallint,to_jsonb(s) \
             FROM game_character_equipment_slots s \
            WHERE s.character_id=encode($1,'hex')::uuid \
         ), carried(item_instance_id,world_id,path,cycle,depth,source_kind,placement) AS ( \
           SELECT item_instance_id,world_id,ARRAY[item_instance_id],false,0,source_kind,placement \
             FROM roots \
           UNION ALL \
           SELECT e.item_instance_id,e.world_id,c.path||e.item_instance_id, \
                  e.item_instance_id=ANY(c.path),c.depth+1,e.source_kind,e.placement \
             FROM carried c JOIN LATERAL ( \
               SELECT e.item_instance_id,e.world_id,1::smallint AS source_kind,to_jsonb(e) AS placement \
                 FROM game_item_container_entries e WHERE e.parent_item_instance_id=c.item_instance_id \
               UNION ALL \
               SELECT e.item_instance_id,e.world_id,2::smallint,to_jsonb(e) \
                 FROM game_item_nested_entries e WHERE e.parent_item_instance_id=c.item_instance_id \
             ) e ON true WHERE NOT c.cycle AND c.depth<9 \
         ), bounded AS MATERIALIZED (SELECT * FROM carried LIMIT 530) \
         SELECT c.item_instance_id::text,i.definition_family,i.definition_production_key, \
                i.definition_revision_ref,i.quantity,i.lifecycle,i.world_id::text AS item_world, \
                c.world_id::text AS custody_world,c.cycle,c.depth, \
                to_jsonb(c)::text AS edge_snapshot,to_jsonb(i)::text AS item_snapshot \
           FROM bounded c LEFT JOIN game_item_instances i USING(item_instance_id) \
          ORDER BY c.item_instance_id,c.source_kind,c.placement::text",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_all(&mut **tx)
    .await?)
}

fn carried_ids(
    rows: &[sqlx::postgres::PgRow],
    world: &[u8; 16],
) -> std::result::Result<BTreeSet<[u8; 16]>, DurabilityError> {
    let mut ids = BTreeSet::new();
    for row in rows {
        if row.try_get::<bool, _>("cycle")?
            || !(0..=8).contains(&row.try_get::<i32, _>("depth")?)
            || row.try_get::<Option<i16>, _>("lifecycle")? != Some(1)
            || row
                .try_get::<Option<String>, _>("item_world")?
                .as_deref()
                .map(uuid_text)
                .transpose()?
                != Some(*world)
            || uuid_text(row.try_get("custody_world")?)? != *world
            || decode_quantity(row)? == 0
            || !ids.insert(uuid_text(row.try_get("item_instance_id")?)?)
        {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    Ok(ids)
}

pub(super) async fn lock_tree(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: &CurrentCharacterItemFence,
    request: &ItemTransferRequest,
) -> Pass<Tree> {
    let character = fence.character_id;
    let world = fence.runtime_scope.world_id();
    let world = world.as_bytes();
    // The Character lock is already held. An occupied MAIN is refused before
    // inspecting its tree, leaving only the bounded equipment set to weigh.
    if load_slot(tx, character).await?.is_some() {
        return Ok(Err(ItemTransferError::Refused(
            ItemTransferRefusal::ContainerSlotOccupied,
        )));
    }
    let source_hint = load_source_read(tx, request.source_item_instance_id, false).await?;
    let Some(source) = &source_hint else {
        return Ok(Err(ItemTransferError::Refused(
            ItemTransferRefusal::SourceNotOnGround,
        )));
    };
    if source.world_id != *world
        || source
            .ground
            .runtime_scope()
            .map_err(|_| DurabilityError::InvalidStoredState)?
            != fence.runtime_scope
    {
        return Ok(Err(ItemTransferError::AuthorityRejected));
    }
    let hint = graph(tx, request.source_item_instance_id).await?;
    let mut height = 0;
    for row in &hint {
        height = height.max(row.try_get::<i32, _>("depth")?);
    }
    if let Err(error) = check_bounds(hint.len(), height)? {
        return Ok(Err(error));
    }
    let carried_hint_rows = carried_rows(tx, character).await?;
    if hint
        .len()
        .checked_add(carried_hint_rows.len())
        .is_none_or(|count| count > 529)
    {
        return Ok(Err(ItemTransferError::CapacityExceeded));
    }
    let carried_hint = carried_ids(&carried_hint_rows, world)?;
    let mut locked = carried_hint.clone();
    // The external corpse parent proves source liveness. Lock it in the same
    // order, but never include it in the incoming tree or its weight.
    if let Some(corpse) = &source.corpse {
        locked.insert(corpse.corpse_item_instance_id);
    }
    for row in &hint {
        locked.insert(uuid_text(row.try_get("item_instance_id")?)?);
    }
    let ids: Vec<Vec<u8>> = locked.iter().map(|id| id.to_vec()).collect();
    sqlx::query(
        "SELECT i.item_instance_id FROM unnest($1::bytea[]) ids(id) \
        JOIN game_item_instances i ON i.item_instance_id=encode(ids.id,'hex')::uuid \
        ORDER BY i.item_instance_id FOR UPDATE OF i",
    )
    .bind(ids)
    .fetch_all(&mut **tx)
    .await?;
    let rows = graph(tx, request.source_item_instance_id).await?;
    if snapshots(&rows)? != snapshots(&hint)?
        || source_hint != load_source_read(tx, request.source_item_instance_id, false).await?
        || snapshots(&carried_hint_rows)? != snapshots(&carried_rows(tx, character).await?)?
    {
        // Abort this pass. A later explicit retry pays the existing RL08 charge.
        return Err(DurabilityError::Unavailable);
    }
    let mut seen = BTreeSet::new();
    let mut ordinals = BTreeSet::new();
    let mut children = BTreeMap::<[u8; 16], usize>::new();
    let mut quantities = BTreeMap::new();
    let mut items = Vec::with_capacity(rows.len());
    for row in &rows {
        if row.try_get::<i16, _>("source_kind")? == 1 {
            return Ok(Err(ItemTransferError::Refused(
                ItemTransferRefusal::ContainerNotEmpty,
            )));
        }
        let id = uuid_text(row.try_get("item_instance_id")?)?;
        let depth: i32 = row.try_get("depth")?;
        if row.try_get::<bool, _>("cyclic")?
            || !seen.insert(id)
            || row.try_get::<Option<i16>, _>("lifecycle")? != Some(1)
            || row
                .try_get::<Option<String>, _>("item_world")?
                .as_deref()
                .map(uuid_text)
                .transpose()?
                != Some(*world)
            || carried_hint.contains(&id)
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        if depth > 0 {
            if row.try_get::<i16, _>("source_kind")? != 2
                || uuid_text(row.try_get("edge_world")?)? != *world
            {
                return Err(DurabilityError::InvalidStoredState);
            }
            let parent = uuid_text(row.try_get("parent_item_instance_id")?)?;
            let ordinal: String = row.try_get("placement_ordinal")?;
            if !ordinals.insert((parent, ordinal)) {
                return Err(DurabilityError::InvalidStoredState);
            }
            *children.entry(parent).or_default() += 1;
        } else if id != request.source_item_instance_id {
            return Err(DurabilityError::InvalidStoredState);
        }
        let quantity = decode_quantity(row)?;
        if quantity == 0 {
            return Err(DurabilityError::InvalidStoredState);
        }
        quantities.insert(id, quantity);
        items.push(InventoryItem {
            item_instance_id: id,
            definition: decode_definition(row)?,
            quantity,
        });
    }
    // Every nonempty tree checks its root as a parent. A singleton still
    // reaches the original plan's ordinary stack/equip refusals.
    if children
        .iter()
        .any(|(parent, count)| *count > 20 || quantities.get(parent) != Some(&1))
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    if children
        .get(&request.source_item_instance_id)
        .copied()
        .unwrap_or(0)
        > usize::try_from(request.item.container_capacity.unwrap_or(0))
            .map_err(|_| DurabilityError::InvalidStoredState)?
    {
        return Ok(Err(ItemTransferError::Refused(
            ItemTransferRefusal::UnsupportedContainerCapacity,
        )));
    }
    Ok(Ok(Tree { items, locked }))
}

// This accessor checks only immutable receipt evidence. Replay remains valid
// after children have legitimately moved out of the promoted MAIN.
pub(super) async fn load_binding(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    transaction: [u8; 16],
) -> std::result::Result<Option<(i16, Vec<u8>)>, DurabilityError> {
    let row = sqlx::query("SELECT tree_size,tree_sha256 FROM game_item_transfer_container_binding(encode($1,'hex')::uuid)")
        .bind(transaction.as_slice()).fetch_optional(&mut **tx).await?;
    row.map(|row| Ok((row.try_get("tree_size")?, row.try_get("tree_sha256")?)))
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_slot_tree_bounds_include_root_and_keep_expansion_separate() {
        assert!(matches!(check_bounds(500, 8), Ok(Ok(()))));
        assert!(matches!(
            check_bounds(501, 8),
            Ok(Err(ItemTransferError::CapacityExceeded))
        ));
        assert!(matches!(
            check_bounds(500, 9),
            Ok(Err(ItemTransferError::CapacityExceeded))
        ));
        assert!(matches!(check_bounds(1, 0), Ok(Ok(()))));
        assert!(matches!(
            check_bounds(0, 0),
            Err(DurabilityError::InvalidStoredState)
        ));
    }
}
