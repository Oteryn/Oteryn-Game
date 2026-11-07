//! PKG5R2B: qualified inventory selection under the fenced equipment owner.
use super::super::item_transfer::BackpackEntry;
use super::super::item_use::ItemUseInventory;
use super::*;
use std::collections::BTreeSet;

/// Crystal 63b93e2c container.cpp:1045-1087 descends before the next sibling.
/// The fenced reader already validates the complete tree and its 500-item/depth bounds.
fn search_order(inventory: &ItemUseInventory) -> Vec<&BackpackEntry> {
    let mut stack: Vec<_> = inventory
        .backpack
        .iter()
        .flat_map(|bag| bag.entries.iter().rev())
        .collect();
    let mut ordered = Vec::new();
    while let Some(entry) = stack.pop() {
        ordered.push(entry);
        stack.extend(inventory.nested.iter().rev().filter_map(|(parent, child)| {
            (*parent == entry.item.item_instance_id).then_some(child)
        }));
    }
    ordered
}

pub(super) fn first_carried_item(
    inventory: &ItemUseInventory,
    definition: &super::super::item_mint::TypedDefinitionRef,
) -> Option<[u8; 16]> {
    choose(search_order(inventory), definition, &BTreeSet::new())
}

fn choose<'a>(
    entries: impl IntoIterator<Item = &'a BackpackEntry>,
    wanted: &super::super::item_mint::TypedDefinitionRef,
    started: &BTreeSet<[u8; 16]>,
) -> Option<[u8; 16]> {
    let mut unused = None;
    for entry in entries {
        if entry.item.definition != *wanted {
            continue;
        }
        let id = entry.item.item_instance_id;
        if started.contains(&id) {
            return Some(id);
        }
        unused.get_or_insert(id);
    }
    unused
}

pub(super) async fn resolve(
    tx: &mut Transaction<'_, Postgres>,
    authority: &EquipmentAuthority,
    snapshot: &EquipmentSnapshot,
    content: &impl EquipmentMoveContent,
    definition: &super::super::item_mint::TypedDefinitionRef,
    slot: u8,
) -> Result<EquipmentIntent> {
    // Owner D1 overrides Crystal's spare-first check before D2 selection.
    if let Some(worn) = snapshot
        .items
        .iter()
        .find(|item| item.slot == slot && item.definition == *definition)
    {
        return Ok(EquipmentIntent::Move {
            item: worn.item_instance_id,
            from_slot: Some(slot),
            to_slot: None,
        });
    }
    let root = sqlx::query("SELECT content_revision,ruleset_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(snapshot.character.as_slice()).fetch_one(&mut **tx).await?;
    let inventory = super::super::item_use::read_use_inventory(
        tx,
        &authority.fence,
        root.try_get("content_revision")?,
        root.try_get("ruleset_revision")?,
    )
    .await
    .map_err(|_| rejected("equipment carried inventory"))?;
    if inventory.backpack.is_none() {
        return Err(rejected("equipment requires actual backpack"));
    }
    let mut ordered = search_order(&inventory);
    // Crystal game.cpp:3743-3751 searches the carried tree first, then the Store Inbox loot
    // pouch. The pouch is an unlimited managed container, so its entries are ordered only to
    // make selection and started-copy preference deterministic.
    let pouch_rows = if ordered
        .iter()
        .any(|entry| entry.item.definition == *definition)
    {
        Vec::new()
    } else {
        sqlx::query("SELECT n.placement_ordinal::text,i.item_instance_id::text,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity FROM game_item_character_inbox_locations l JOIN game_item_instances p ON p.item_instance_id=l.item_instance_id JOIN game_item_nested_entries n ON n.parent_item_instance_id=p.item_instance_id JOIN game_item_instances i ON i.item_instance_id=n.item_instance_id WHERE l.character_id=encode($1,'hex')::uuid AND l.world_id=encode($2,'hex')::uuid AND p.definition_production_key='oteryn:item.tibia.i23721' AND i.lifecycle=1 ORDER BY n.placement_ordinal DESC LIMIT 500 FOR UPDATE OF i")
        .bind(snapshot.character.as_slice()).bind(authority.world.as_slice()).fetch_all(&mut **tx).await?
    };
    let pouch_entries: Vec<_> = pouch_rows
        .into_iter()
        .map(|row| {
            Ok(BackpackEntry {
                placement_ordinal: number(row.try_get("placement_ordinal")?)?,
                item: super::super::item_transfer::InventoryItem {
                    item_instance_id: uuid(row.try_get("item_instance_id")?)?,
                    definition: super::super::item_mint::TypedDefinitionRef {
                        family: row.try_get("definition_family")?,
                        production_key: row.try_get("definition_production_key")?,
                        revision_ref: row.try_get("definition_revision_ref")?,
                    },
                    quantity: u32::try_from(row.try_get::<i64, _>("quantity")?)
                        .map_err(|_| rejected("equipment pouch quantity"))?,
                },
            })
        })
        .collect::<Result<_>>()?;
    ordered.extend(pouch_entries.iter());
    let mut started = BTreeSet::new();
    if let Some(full) = content.full_timed_values(definition) {
        let ids: Vec<_> = ordered
            .iter()
            .filter(|e| e.item.definition == *definition)
            .map(|e| {
                e.item
                    .item_instance_id
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            })
            .collect();
        let rows = sqlx::query("SELECT item_instance_id::text,charges,remaining_ms FROM game_item_timed_states WHERE item_instance_id IN (SELECT value::uuid FROM jsonb_array_elements_text($1::jsonb)) ORDER BY item_instance_id FOR UPDATE")
            .bind(sqlx::types::Json(ids)).fetch_all(&mut **tx).await?;
        for row in rows {
            let charges: Option<i32> = row.try_get("charges")?;
            let remaining: Option<i64> = row.try_get("remaining_ms")?;
            let values = crate::domain::timed_item::TimedValues::new(
                charges
                    .map(u32::try_from)
                    .transpose()
                    .map_err(|_| rejected("equipment timed charges"))?,
                remaining
                    .map(u64::try_from)
                    .transpose()
                    .map_err(|_| rejected("equipment timed remaining"))?,
            )
            .map_err(|_| rejected("equipment timed values"))?;
            if is_started(values, full)? {
                started.insert(uuid(row.try_get("item_instance_id")?)?);
            }
        }
    }
    let item =
        choose(ordered, definition, &started).ok_or(rejected("equipment actual live item"))?;
    Ok(EquipmentIntent::Move {
        item,
        from_slot: None,
        to_slot: Some(slot),
    })
}

fn is_started(
    values: crate::domain::timed_item::TimedValues,
    full: crate::domain::timed_item::TimedValues,
) -> Result<bool> {
    let charges = match (values.charges(), full.charges()) {
        (None, None) => false,
        (Some(value), Some(full)) if value <= full => value < full,
        _ => return Err(rejected("equipment timed budget")),
    };
    let time = match (values.remaining_ms(), full.remaining_ms()) {
        (None, None) => false,
        (Some(value), Some(full)) if value > 0 && value <= full => value < full,
        _ => return Err(rejected("equipment timed budget")),
    };
    Ok(charges || time)
}

/// Reprove the entire source ancestry, locking members in UUID order under the held root.
pub(super) async fn lock_nested_source(
    tx: &mut Transaction<'_, Postgres>,
    authority: &EquipmentAuthority,
    item: [u8; 16],
    backpack: [u8; 16],
) -> Result<([u8; 16], u64)> {
    let rows=sqlx::query("WITH RECURSIVE path(item,depth) AS (SELECT encode($1,'hex')::uuid,0 UNION ALL SELECT e.parent_item_instance_id,p.depth+1 FROM path p JOIN (SELECT item_instance_id,parent_item_instance_id FROM game_item_container_entries UNION ALL SELECT item_instance_id,parent_item_instance_id FROM game_item_nested_entries) e ON e.item_instance_id=p.item WHERE p.depth<9) SELECT i.item_instance_id::text,p.depth FROM path p JOIN game_item_instances i ON i.item_instance_id=p.item WHERE i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 ORDER BY i.item_instance_id LIMIT 10 FOR UPDATE OF i")
        .bind(item.as_slice()).bind(authority.world.as_slice()).fetch_all(&mut **tx).await?;
    let backpack_path = (3..=9).contains(&rows.len())
        && rows.iter().any(|row| {
            match (
                row.try_get::<String, _>("item_instance_id"),
                row.try_get::<i32, _>("depth"),
            ) {
                (Ok(id), Ok(depth)) => {
                    uuid(id).ok() == Some(backpack)
                        && usize::try_from(depth).ok() == Some(rows.len() - 1)
                }
                _ => false,
            }
        });
    let pouch_path = if rows.len() == 2 {
        let root = rows
            .iter()
            .find_map(|row| {
                (row.try_get::<i32, _>("depth").ok()? == 1)
                    .then(|| row.try_get::<String, _>("item_instance_id").ok())
                    .flatten()
            })
            .and_then(|id| uuid(id).ok());
        if let Some(root) = root {
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM game_item_character_inbox_locations l JOIN game_item_instances p ON p.item_instance_id=l.item_instance_id WHERE l.character_id=encode($1,'hex')::uuid AND l.world_id=encode($2,'hex')::uuid AND l.item_instance_id=encode($3,'hex')::uuid AND p.definition_production_key='oteryn:item.tibia.i23721')")
                .bind(authority.fence.character_id.as_bytes().as_slice()).bind(authority.world.as_slice()).bind(root.as_slice()).fetch_one(&mut **tx).await?
        } else {
            false
        }
    } else {
        false
    };
    if !backpack_path && !pouch_path {
        return Err(rejected("equipment carried ancestry"));
    }
    let source=sqlx::query("SELECT e.parent_item_instance_id::text,e.placement_ordinal::text FROM game_item_nested_entries e CROSS JOIN LATERAL game_item_loop_owner(e.parent_item_instance_id) o WHERE e.item_instance_id=encode($1,'hex')::uuid AND e.world_id=encode($2,'hex')::uuid AND o.owner_character=encode($3,'hex')::uuid AND o.depth BETWEEN 1 AND 7 UNION ALL SELECT e.parent_item_instance_id::text,e.placement_ordinal::text FROM game_item_nested_entries e JOIN game_item_character_inbox_locations l ON l.item_instance_id=e.parent_item_instance_id JOIN game_item_instances p ON p.item_instance_id=l.item_instance_id WHERE e.item_instance_id=encode($1,'hex')::uuid AND e.world_id=encode($2,'hex')::uuid AND l.character_id=encode($3,'hex')::uuid AND p.definition_production_key='oteryn:item.tibia.i23721' LIMIT 1")
        .bind(item.as_slice()).bind(authority.world.as_slice()).bind(authority.fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?.ok_or(rejected("equipment carried ancestry"))?;
    Ok((
        uuid(source.try_get("parent_item_instance_id")?)?,
        number(source.try_get("placement_ordinal")?)?,
    ))
}

/// Crystal game.cpp:3755 routes toggled equipment through managed containers.
/// Reuse the quick-loot equipment category, falling back to its unassigned target.
pub(super) async fn managed_destination(
    tx: &mut Transaction<'_, Postgres>,
    authority: &EquipmentAuthority,
    content: &impl EquipmentMoveContent,
    backpack: [u8; 16],
    moving: [u8; 16],
) -> Result<Option<([u8; 16], u64)>> {
    use oteryn_gameplay_items::quick_loot::{LootCategory, Recipient};
    let character = authority.fence.character_id.as_bytes();
    let settings = super::super::character_quick_loot::load(tx, character).await?;
    let target = settings
        .category_targets
        .get(&LootCategory::Equipment)
        .copied()
        .unwrap_or(settings.unassigned_target);
    let Recipient::Container(target) = target else {
        return Ok(None);
    };
    if target == backpack {
        return Ok(None);
    }
    let root = sqlx::query("SELECT content_revision,ruleset_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(character.as_slice()).fetch_one(&mut **tx).await?;
    let inventory = super::super::item_use::read_use_inventory(
        tx,
        &authority.fence,
        root.try_get("content_revision")?,
        root.try_get("ruleset_revision")?,
    )
    .await
    .map_err(|_| rejected("equipment carried inventory"))?;
    let entry = search_order(&inventory)
        .into_iter()
        .find(|entry| entry.item.item_instance_id == target)
        .ok_or(rejected("equipment managed container owner"))?;
    let policy = content
        .equipment_policy(
            &entry.item.definition.production_key,
            &entry.item.definition.revision_ref,
        )
        .ok_or(rejected("equipment managed container policy"))?;
    if policy.source_digest() != authority.content_digest {
        return Err(rejected("equipment managed container pin"));
    }
    let capacity = policy
        .container_capacity()
        .ok_or(rejected("equipment managed container capacity"))?;
    let row = sqlx::query("SELECT o.depth FROM game_item_instances i CROSS JOIN LATERAL game_item_loop_owner(i.item_instance_id) o WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND o.owner_character=encode($3,'hex')::uuid AND o.depth BETWEEN 1 AND 7 FOR UPDATE OF i")
        .bind(target.as_slice()).bind(authority.world.as_slice()).bind(character.as_slice())
        .fetch_optional(&mut **tx).await?;
    let destination_depth: i32 = row
        .ok_or(rejected("equipment managed container ancestry"))?
        .try_get("depth")?;
    // An equipped quiver can itself have contents. Moving its entire subtree must
    // preserve BAGS-0's depth and carried-tree bounds, just as a loop move does.
    let subtree = sqlx::query("WITH RECURSIVE tree(item,depth) AS (SELECT encode($1,'hex')::uuid,0 UNION ALL SELECT e.item_instance_id,t.depth+1 FROM tree t JOIN game_item_nested_entries e ON e.parent_item_instance_id=t.item WHERE t.depth<9) SELECT depth FROM tree LIMIT 501")
        .bind(moving.as_slice()).fetch_all(&mut **tx).await?;
    let carried_count = inventory
        .backpack
        .iter()
        .map(|bag| bag.entries.len())
        .sum::<usize>()
        + inventory.nested.len();
    if carried_count + subtree.len() > 500
        || subtree.iter().any(|row| {
            row.try_get::<i32, _>("depth")
                .map_or(true, |depth| destination_depth + 1 + depth > 8)
        })
    {
        return Err(rejected("equipment backpack capacity"));
    }
    let row = sqlx::query("SELECT count(*) AS entries,COALESCE(max(placement_ordinal),0)::text AS maximum FROM game_item_nested_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid")
        .bind(target.as_slice()).fetch_one(&mut **tx).await?;
    if row.try_get::<i64, _>("entries")? >= i64::from(capacity.min(20)) {
        return Err(rejected("equipment backpack capacity"));
    }
    let ordinal = number(row.try_get("maximum")?)?
        .checked_add(1)
        .ok_or(rejected("equipment backpack ordinal exhausted"))?;
    Ok(Some((target, ordinal)))
}

/// Only a hotkey may vacate a conflicting opposite hand. Manual moves retain refusal.
/// Crystal game.cpp:3770-3791; the caller has already proved the claims conflict.
pub(super) fn displaces_other_hand(hotkey: bool, target: u8, occupied: u8) -> bool {
    hotkey && matches!((target, occupied), (5, 6) | (6, 5))
}

pub(super) async fn main_vacancy(
    tx: &mut Transaction<'_, Postgres>,
    authority: &EquipmentAuthority,
    content: &impl EquipmentMoveContent,
    backpack: [u8; 16],
) -> Result<()> {
    main_room(tx, authority, content, backpack, None, 1).await
}

pub(super) async fn main_room(
    tx: &mut Transaction<'_, Postgres>,
    authority: &EquipmentAuthority,
    content: &impl EquipmentMoveContent,
    backpack: [u8; 16],
    source: Option<[u8; 16]>,
    added: usize,
) -> Result<()> {
    let row=sqlx::query("SELECT definition_production_key,definition_revision_ref,(SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id) AS entries,game_item_loop_tree_size(encode($2,'hex')::uuid) AS tree,EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=encode($3,'hex')::uuid AND parent_item_instance_id=i.item_instance_id) AS direct_source,EXISTS(SELECT 1 FROM game_item_loop_owner(encode($3,'hex')::uuid) WHERE owner_character=encode($2,'hex')::uuid AND depth>0) AS tree_source FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(backpack.as_slice()).bind(authority.fence.character_id.as_bytes().as_slice()).bind(source.map(|id| id.to_vec())).fetch_one(&mut **tx).await?;
    let policy = content
        .equipment_policy(
            &row.try_get::<String, _>("definition_production_key")?,
            &row.try_get::<String, _>("definition_revision_ref")?,
        )
        .ok_or(rejected("equipment actual backpack active policy"))?;
    if policy.source_digest() != authority.content_digest {
        return Err(rejected("equipment backpack profile pin"));
    }
    let capacity = policy
        .container_capacity()
        .ok_or(rejected("equipment unknown backpack capacity"))?;
    let entries = usize::try_from(row.try_get::<i64, _>("entries")?)
        .map_err(|_| rejected("equipment backpack capacity"))?;
    let tree = usize::try_from(row.try_get::<i64, _>("tree")?)
        .map_err(|_| rejected("equipment backpack capacity"))?;
    if !room_for(
        entries,
        usize::from(row.try_get::<bool, _>("direct_source")?),
        added,
        usize::from(capacity.min(20)),
    ) || !room_for(
        tree,
        usize::from(row.try_get::<bool, _>("tree_source")?),
        added,
        500,
    ) {
        return Err(rejected("equipment backpack capacity"));
    }
    Ok(())
}

fn room_for(entries: usize, removed: usize, added: usize, capacity: usize) -> bool {
    entries
        .checked_sub(removed)
        .and_then(|n| n.checked_add(added))
        .is_some_and(|n| n <= capacity)
}

#[cfg(test)]
mod tests {
    use super::super::super::item_transfer::{CharacterBackpack, InventoryItem};
    use super::*;
    fn definition() -> super::super::super::item_mint::TypedDefinitionRef {
        super::super::super::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: "test.sword".into(),
            revision_ref: "r1".into(),
        }
    }
    fn entry(id: u8, ordinal: u64) -> BackpackEntry {
        BackpackEntry {
            placement_ordinal: ordinal,
            item: InventoryItem {
                item_instance_id: [id; 16],
                definition: definition(),
                quantity: 1,
            },
        }
    }
    #[test]
    fn hotkey_swap_opposite_hands_only_and_manual_conflicts_stay_refused() {
        for (target, occupied) in [(5, 6), (6, 5)] {
            assert!(displaces_other_hand(true, target, occupied));
            assert!(!displaces_other_hand(false, target, occupied));
        }
        for target in [1, 2, 3, 4, 5, 6, 7, 8, 10] {
            assert!(!displaces_other_hand(true, target, target));
            assert!(!displaces_other_hand(true, target, 10));
        }
    }
    #[test]
    fn hotkey_swap_reserves_all_displaced_items_before_any_move() {
        assert!(room_for(19, 1, 2, 20));
        assert!(!room_for(20, 1, 2, 20));
        assert!(!room_for(19, 0, 2, 20));
        assert!(room_for(20, 1, 1, 20));
        assert!(!room_for(500, 1, 2, 500));
        assert!(!room_for(0, 1, 2, 20));
    }
    #[test]
    fn nested_search_descends_before_sibling_and_preserves_display_order() {
        let inventory = ItemUseInventory {
            content_revision: "c".into(),
            ruleset_revision: "r".into(),
            equipment: vec![],
            fluids: Default::default(),
            backpack: Some(CharacterBackpack {
                backpack: InventoryItem {
                    item_instance_id: [1; 16],
                    definition: definition(),
                    quantity: 1,
                },
                entries: vec![entry(2, 3), entry(3, 2), entry(4, 1)],
                fluids: Default::default(),
            }),
            nested: vec![
                ([2; 16], entry(5, 2)),
                ([2; 16], entry(6, 1)),
                ([5; 16], entry(7, 1)),
            ],
            keys: vec![],
        };
        assert_eq!(
            search_order(&inventory)
                .iter()
                .map(|e| e.item.item_instance_id[0])
                .collect::<Vec<_>>(),
            [2, 5, 7, 6, 3, 4]
        );
    }
    #[test]
    fn started_copy_precedes_unused_and_first_started_wins() {
        let entries = [entry(1, 3), entry(2, 2), entry(3, 1)];
        for (started, expected) in [(vec![], 1), (vec![[2; 16]], 2), (vec![[2; 16], [3; 16]], 2)] {
            assert_eq!(
                choose(&entries, &definition(), &started.into_iter().collect()),
                Some([expected; 16])
            );
        }
        assert_eq!(
            choose(&entries, &definition(), &BTreeSet::from([[9; 16]])),
            Some([1; 16])
        );
    }
    #[test]
    fn started_means_less_time_or_charges_than_qualified_full() -> Result<()> {
        use crate::domain::timed_item::TimedValues;
        let values = |c, t| TimedValues::new(c, t).map_err(|_| rejected("test values"));
        for full in [
            values(Some(100), None)?,
            values(None, Some(100))?,
            values(Some(100), Some(100))?,
        ] {
            assert!(!is_started(full, full)?);
        }
        assert!(is_started(
            values(Some(99), None)?,
            values(Some(100), None)?
        )?);
        assert!(is_started(
            values(None, Some(99))?,
            values(None, Some(100))?
        )?);
        assert!(is_started(
            values(Some(99), Some(100))?,
            values(Some(100), Some(100))?
        )?);
        assert!(is_started(values(Some(101), None)?, values(Some(100), None)?).is_err());
        Ok(())
    }
}
