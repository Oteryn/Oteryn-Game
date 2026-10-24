//! Fenced carried inventory snapshot and item-use PREPARE plans (BAGS-0 §8).

use super::super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::super::item_loop_move::{ITEM_LOOP_DEPTH_MAX, ITEM_LOOP_TREE_ITEMS_MAX};
use super::super::item_transfer::{CharacterBackpack, InventoryItem};
use super::*;
use oteryn_gameplay_items::hotkey_search::{HotkeyRefusal, first_unreserved};

/// BAGS-0 §8 fallback until a complete runtime inventory projection exists. One fenced read
/// includes closed bags; entries keep their parent and display ordinal. No partial tree is usable.
#[derive(Debug)]
pub struct ItemUseInventory {
    pub content_revision: String,
    pub ruleset_revision: String,
    pub equipment: Vec<InventoryItem>,
    pub backpack: Option<CharacterBackpack>,
    pub nested: Vec<([u8; 16], BackpackEntry)>,
}

impl ItemUseInventory {
    /// ITEM-USE-0 §11: reserve a compatible main-backpack stack, a fresh main entry, or a
    /// missing flask. Retiring a nested source never frees a main entry. This PREPARE plan
    /// grants no authority: the fenced writer checks it again under the item and owner locks.
    pub fn plan_flask(
        &self,
        source: &BackpackEntry,
        definition: TypedDefinitionRef,
        fresh_id: impl FnOnce() -> Option<[u8; 16]>,
    ) -> Result<ItemUseFlaskPlan> {
        let entries = &self
            .backpack
            .as_ref()
            .ok_or(ItemUseError::StaleState)?
            .entries;
        if let Some(stack) = entries.iter().find(|entry| {
            entry.item.definition == definition
                && entry.item.quantity < GAMEITEM01_STACK_QUANTITY_MAX
        }) {
            return Ok(ItemUseFlaskPlan::Stack {
                item_instance_id: stack.item.item_instance_id,
                definition,
                quantity_before: stack.item.quantity,
            });
        }
        let retiring = usize::from(
            source.item.quantity == 1
                && entries
                    .iter()
                    .any(|entry| entry.item.item_instance_id == source.item.item_instance_id),
        );
        let count = entries
            .len()
            .checked_add(1)
            .and_then(|n| n.checked_sub(retiring))
            .and_then(|n| u32::try_from(n).ok())
            .ok_or(ItemUseError::InvalidInput)?;
        if count > GAMEITEM01_CONTAINER_ENTRIES_MAX {
            return Ok(ItemUseFlaskPlan::Missing);
        }
        let placement_ordinal = entries
            .iter()
            .map(|entry| entry.placement_ordinal)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(ItemUseError::InvalidInput)?;
        let item_instance_id = fresh_id().ok_or(ItemUseError::InvalidInput)?;
        Ok(ItemUseFlaskPlan::New {
            item_instance_id,
            definition,
            placement_ordinal,
        })
    }

    /// BAGS-0 §8: search this complete fenced snapshot in wire/display order, including
    /// closed bags. The pure items rule owns ordering, reservation skips and the whole-input
    /// bound; an equipment match refuses without falling through to a later stack.
    pub fn first_unreserved(
        &self,
        wanted: &TypedDefinitionRef,
        reserved: &[[u8; 16]],
    ) -> std::result::Result<[u8; 16], HotkeyRefusal> {
        first_unreserved(
            self.equipment.iter(),
            self.backpack
                .iter()
                .flat_map(|bag| &bag.entries)
                .map(|entry| &entry.item),
            |item| {
                self.nested
                    .iter()
                    .filter(|(parent, _)| *parent == item.item_instance_id)
                    .map(|(_, entry)| &entry.item)
                    .collect()
            },
            |item| &item.definition,
            wanted,
            |item| reserved.contains(&item.item_instance_id),
        )
        .map(|item| item.item_instance_id)
    }
}

impl DurabilityRoot {
    /// Read the complete carried tree and equipment in one bounded snapshot under the current
    /// item fence. The Character root lock serializes tree changes, including closed bags.
    pub async fn read_item_use_inventory(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
    ) -> Result<ItemUseInventory> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemUseError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let root = match assert_item_only_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => root,
                        Err(CharacterProgressionError::Unavailable(error)) => return Err(error),
                        Err(_) => return Ok(Err(ItemUseError::AuthorityRejected)),
                    };
                    let inventory = read_use_inventory(
                        &mut tx,
                        &fence,
                        root.content_revision,
                        root.ruleset_revision,
                    )
                    .await;
                    match inventory {
                        Ok(inventory) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(inventory))
                        }
                        Err(ItemUseError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}

pub(super) async fn read_use_inventory(
    connection: &mut PgConnection,
    fence: &CurrentCharacterItemFence,
    content_revision: String,
    ruleset_revision: String,
) -> Result<ItemUseInventory> {
    let RuntimeScopeRefV1::Channel { world_id, .. } = fence.runtime_scope else {
        return Err(ItemUseError::AuthorityRejected);
    };
    // At most 500 tree items plus nine slots. Read one overflow sentinel and one excess
    // depth level, refusing the whole read rather than silently truncating it.
    let rows = sqlx::query(
        "WITH RECURSIVE tree(item, world, parent, ordinal, depth) AS ( \
           SELECT item_instance_id, world_id, NULL::uuid, 0::numeric, 0 \
             FROM game_item_container_slots WHERE character_id = encode($1,'hex')::uuid \
           UNION ALL \
           SELECT e.item_instance_id, e.world_id, e.parent_item_instance_id, \
                  e.placement_ordinal, t.depth + 1 FROM tree t JOIN ( \
             SELECT item_instance_id, world_id, parent_item_instance_id, placement_ordinal \
               FROM game_item_container_entries WHERE character_id = encode($1,'hex')::uuid \
             UNION ALL \
             SELECT item_instance_id, world_id, parent_item_instance_id, placement_ordinal \
               FROM game_item_nested_entries \
           ) e ON e.parent_item_instance_id = t.item WHERE t.depth <= $2 \
         ), carried AS ( \
           SELECT *, 0::smallint AS slot FROM tree \
           UNION ALL \
           SELECT item_instance_id, world_id, NULL::uuid, 0::numeric, 0, slot \
             FROM game_character_equipment_slots WHERE character_id = encode($1,'hex')::uuid \
         ) SELECT c.item::text, c.world::text AS custody_world, c.parent::text, \
                  c.ordinal::text, c.depth, c.slot, i.world_id::text AS item_world, \
                  i.definition_family, i.definition_production_key, i.definition_revision_ref, \
                  i.quantity, i.lifecycle \
             FROM carried c JOIN game_item_instances i ON i.item_instance_id = c.item \
            ORDER BY c.slot, c.depth, c.ordinal DESC LIMIT 510",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(ITEM_LOOP_DEPTH_MAX)
    .fetch_all(connection)
    .await?;
    let mut inventory = ItemUseInventory {
        content_revision,
        ruleset_revision,
        equipment: Vec::new(),
        backpack: None,
        nested: Vec::new(),
    };
    let mut seen = std::collections::BTreeSet::new();
    let mut tree_items = 0_i64;
    for row in rows {
        let id = uuid_text(&row.try_get::<String, _>("item")?)?;
        let slot: i16 = row.try_get("slot")?;
        let depth: i32 = row.try_get("depth")?;
        if !seen.insert(id)
            || row.try_get::<i16, _>("lifecycle")? != 1
            || uuid_text(&row.try_get::<String, _>("item_world")?)? != *world_id.as_bytes()
            || uuid_text(&row.try_get::<String, _>("custody_world")?)? != *world_id.as_bytes()
            || depth > ITEM_LOOP_DEPTH_MAX
            || !(0..=9).contains(&slot)
        {
            return Err(ItemUseError::StaleState);
        }
        let item = InventoryItem {
            item_instance_id: id,
            definition: TypedDefinitionRef {
                family: row.try_get("definition_family")?,
                production_key: row.try_get("definition_production_key")?,
                revision_ref: row.try_get("definition_revision_ref")?,
            },
            quantity: u32::try_from(row.try_get::<i64, _>("quantity")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        };
        if slot != 0 {
            inventory.equipment.push(item);
            if inventory.equipment.len() > 9 {
                return Err(ItemUseError::StaleState);
            }
            continue;
        }
        tree_items += 1;
        if tree_items > ITEM_LOOP_TREE_ITEMS_MAX {
            return Err(ItemUseError::StaleState);
        }
        if depth == 0 {
            if inventory.backpack.is_some() {
                return Err(ItemUseError::StaleState);
            }
            inventory.backpack = Some(CharacterBackpack {
                backpack: item,
                entries: Vec::new(),
            });
        } else {
            let parent = uuid_text(&row.try_get::<String, _>("parent")?)?;
            let entry = BackpackEntry {
                item,
                placement_ordinal: numeric_u64(&row, "ordinal")?,
            };
            if depth == 1 {
                inventory
                    .backpack
                    .as_mut()
                    .ok_or(ItemUseError::StaleState)?
                    .entries
                    .push(entry);
            } else {
                inventory.nested.push((parent, entry));
            }
        }
    }
    Ok(inventory)
}
