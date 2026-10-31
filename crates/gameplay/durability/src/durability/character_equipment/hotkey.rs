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
    let ordered = search_order(&inventory);
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
    if !(3..=9).contains(&rows.len())
        || !rows.iter().any(|row| {
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
        })
    {
        return Err(rejected("equipment carried ancestry"));
    }
    let source=sqlx::query("SELECT e.parent_item_instance_id::text,e.placement_ordinal::text FROM game_item_nested_entries e CROSS JOIN LATERAL game_item_loop_owner(e.parent_item_instance_id) o WHERE e.item_instance_id=encode($1,'hex')::uuid AND e.world_id=encode($2,'hex')::uuid AND o.owner_character=encode($3,'hex')::uuid AND o.depth BETWEEN 1 AND 7")
        .bind(item.as_slice()).bind(authority.world.as_slice()).bind(authority.fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?.ok_or(rejected("equipment carried ancestry"))?;
    Ok((
        uuid(source.try_get("parent_item_instance_id")?)?,
        number(source.try_get("placement_ordinal")?)?,
    ))
}

pub(super) async fn main_vacancy(
    tx: &mut Transaction<'_, Postgres>,
    authority: &EquipmentAuthority,
    content: &impl EquipmentMoveContent,
    backpack: [u8; 16],
) -> Result<()> {
    let row=sqlx::query("SELECT definition_production_key,definition_revision_ref,(SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id) AS entries FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(backpack.as_slice()).fetch_one(&mut **tx).await?;
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
    if row.try_get::<i64, _>("entries")? >= i64::from(capacity.min(20)) {
        return Err(rejected("equipment backpack capacity"));
    }
    Ok(())
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
