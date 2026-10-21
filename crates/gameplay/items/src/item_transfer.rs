//! Pure D80-D83 transfer placement and capacity admission.

pub use crate::stacking::stack_maximum;
use oteryn_gameplay_domain::domain::item_definition::{ItemDefinitionFacts, TypedDefinitionRef};

/// GAMEITEM01-STACK-QUANTITY-MAX (D82).
pub const GAMEITEM01_STACK_QUANTITY_MAX: u32 = 100;
/// GAMEITEM01-CONTAINER-ENTRIES-MAX: a main backpack with a larger declared
/// capacity waits for a new decision.
pub const GAMEITEM01_CONTAINER_ENTRIES_MAX: u32 = 20;
/// GAMEITEM01-PLACEMENT-DEPTH: only direct entries of the main backpack.
pub const GAMEITEM01_PLACEMENT_DEPTH: u32 = 1;
/// GAMEITEM01-REACHABLE-ITEMS: the main backpack and its entries.
pub const GAMEITEM01_REACHABLE_ITEMS: u32 = 1 + GAMEITEM01_CONTAINER_ENTRIES_MAX;

/// D133: the exclusive top-damage window after a corpse's `materialized_at`.
pub const CORPSE_EXCLUSIVE_WINDOW_MS: i64 = 10_000;

/// Pure D133 gate: may `requester` take an entry out of a corpse at `now`?
/// The top-damage Character always may; anyone else only at or after
/// `materialized_at + 10 s` (the deadline itself is already open). Times are
/// database-clock milliseconds; an overflowing deadline stays closed.
#[must_use]
pub fn corpse_window_allows(
    requester: [u8; 16],
    top_damage: [u8; 16],
    materialized_at_unix_ms: i64,
    now_unix_ms: i64,
) -> bool {
    requester == top_damage
        || materialized_at_unix_ms
            .checked_add(CORPSE_EXCLUSIVE_WINDOW_MS)
            .is_some_and(|deadline| now_unix_ms >= deadline)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemTransferDestination {
    /// CharacterEquipment slot `container`.
    ContainerSlot,
    /// A direct entry of the main backpack, including the D83 merge shapes.
    MainBackpack,
}

/// Pickup refusal reasons (B3 §5). A refusal writes nothing and the item
/// stays on Ground.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemTransferRefusal {
    WeightUnknown,
    WeightGenerationMismatch,
    WeightDefinitionMismatch,
    Overweight,
    /// The source is neither a live Ground item nor a direct entry of a live
    /// corpse container (for example already picked up).
    SourceNotOnGround,
    /// The supplied facts do not describe the stored item or main backpack.
    DefinitionMismatch,
    /// Unknown stack class (D82: fail closed).
    UnknownStackClass,
    /// A proven stack maximum of 0 or above GAMEITEM01-STACK-QUANTITY-MAX.
    UnsupportedStackMaximum,
    /// The stored quantity exceeds the definition's stack maximum.
    QuantityAboveStackMaximum,
    /// The item has no complete `container`-slot equip pattern.
    NotContainerSlotEquippable,
    ContainerSlotOccupied,
    /// A moved container must be empty (DUR03-RL-05 = 0).
    ContainerNotEmpty,
    /// No main backpack is equipped.
    NoMainBackpack,
    /// A main backpack capacity of 0 or above GAMEITEM01-CONTAINER-ENTRIES-MAX.
    UnsupportedContainerCapacity,
    /// No free entry, and no full merge is possible.
    MainBackpackFull,
    /// D133: the source is an entry of a corpse still inside its 10 s
    /// exclusivity window and the requester is not its top-damage Character.
    CorpseExclusiveWindow,
    /// D134: the source item is itself a corpse. A corpse is never a TRANSFER
    /// source, whether or not it currently holds entries.
    CorpseNotPickupable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContainerEntryPosition {
    pub parent_item_instance_id: [u8; 16],
    pub placement_ordinal: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferReceiver {
    pub item_instance_id: [u8; 16],
    pub quantity_before: u32,
    pub quantity_after: u32,
}

/// One item as stored (live).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryItem {
    pub item_instance_id: [u8; 16],
    pub definition: TypedDefinitionRef,
    pub quantity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackpackEntry {
    pub item: InventoryItem,
    pub placement_ordinal: u64,
}

/// The main backpack and its direct entries in display order (newest first).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterBackpack {
    pub backpack: InventoryItem,
    pub entries: Vec<BackpackEntry>,
}

/// Where a live TRANSFER source currently is (D3-5). A live item is on at
/// most one of these, and neither ever becomes the other: Ground items are
/// never re-parented into a corpse, and corpse entries leave only by TRANSFER
/// or decay, so a read here stays true until the item stops being a source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemSourceLocation {
    Ground,
    CorpseEntry { corpse_item_instance_id: [u8; 16] },
}

/// Materialized D83 plan over the authoritative before-state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferPlan {
    ContainerSlot,
    NewEntry {
        position: ContainerEntryPosition,
    },
    FullMerge {
        receiver: TransferReceiver,
        receiver_position: ContainerEntryPosition,
    },
    TopUp {
        receiver: TransferReceiver,
        receiver_position: ContainerEntryPosition,
        position: ContainerEntryPosition,
        source_after: u32,
    },
}

/// Authoritative before-state read under the `character_root` row lock.
pub struct PlanInput<'a> {
    pub destination: ItemTransferDestination,
    pub item: &'a ItemDefinitionFacts,
    pub backpack: Option<&'a ItemDefinitionFacts>,
    pub source: &'a InventoryItem,
    pub source_has_entries: bool,
    pub slot: Option<&'a InventoryItem>,
    pub entries: &'a [BackpackEntry],
}

/// Pure D80-D83 admission and plan. Never selects a receiver by UUID.
pub fn plan_transfer(
    input: &PlanInput<'_>,
) -> std::result::Result<TransferPlan, ItemTransferRefusal> {
    use ItemTransferRefusal as Refusal;
    if input.item.definition != input.source.definition {
        return Err(Refusal::DefinitionMismatch);
    }
    let (maximum, stackable) = stack_maximum(input.item.stack)?;
    if input.source.quantity > maximum {
        return Err(Refusal::QuantityAboveStackMaximum);
    }
    if input.source_has_entries {
        return Err(Refusal::ContainerNotEmpty);
    }
    match input.destination {
        ItemTransferDestination::ContainerSlot => {
            let Some(capacity) = input.item.container_capacity else {
                return Err(Refusal::NotContainerSlotEquippable);
            };
            if !input.item.container_slot_equip_pattern {
                return Err(Refusal::NotContainerSlotEquippable);
            }
            if capacity == 0 || capacity > GAMEITEM01_CONTAINER_ENTRIES_MAX {
                return Err(Refusal::UnsupportedContainerCapacity);
            }
            if input.slot.is_some() {
                return Err(Refusal::ContainerSlotOccupied);
            }
            Ok(TransferPlan::ContainerSlot)
        }
        ItemTransferDestination::MainBackpack => {
            let slot = input.slot.ok_or(Refusal::NoMainBackpack)?;
            let backpack = input.backpack.ok_or(Refusal::DefinitionMismatch)?;
            if backpack.definition != slot.definition {
                return Err(Refusal::DefinitionMismatch);
            }
            let capacity = backpack
                .container_capacity
                .ok_or(Refusal::DefinitionMismatch)?;
            if capacity == 0 || capacity > GAMEITEM01_CONTAINER_ENTRIES_MAX {
                return Err(Refusal::UnsupportedContainerCapacity);
            }
            let count = u32::try_from(input.entries.len()).unwrap_or(u32::MAX);
            let free = count < capacity;
            let next = input
                .entries
                .iter()
                .map(|entry| entry.placement_ordinal)
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or(Refusal::MainBackpackFull)?;
            let position = ContainerEntryPosition {
                parent_item_instance_id: slot.item_instance_id,
                placement_ordinal: next,
            };
            let quantity = input.source.quantity;
            // D83: the compatible stack with room that comes first in display
            // order (the highest ordinal). Compatible = same definition key and
            // revision; quantities may differ.
            let receiver = stackable
                .then(|| {
                    input
                        .entries
                        .iter()
                        .filter(|entry| {
                            entry.item.definition == input.source.definition
                                && entry.item.quantity < maximum
                        })
                        .max_by_key(|entry| entry.placement_ordinal)
                })
                .flatten();
            if let Some(entry) = receiver {
                let receiver_position = ContainerEntryPosition {
                    parent_item_instance_id: slot.item_instance_id,
                    placement_ordinal: entry.placement_ordinal,
                };
                let (moved, remaining) =
                    crate::stacking::plan_stack(quantity, maximum, entry.item.quantity);
                if remaining == 0 {
                    return Ok(TransferPlan::FullMerge {
                        receiver: TransferReceiver {
                            item_instance_id: entry.item.item_instance_id,
                            quantity_before: entry.item.quantity,
                            quantity_after: entry.item.quantity + moved,
                        },
                        receiver_position,
                    });
                }
                if !free {
                    return Err(Refusal::MainBackpackFull);
                }
                return Ok(TransferPlan::TopUp {
                    receiver: TransferReceiver {
                        item_instance_id: entry.item.item_instance_id,
                        quantity_before: entry.item.quantity,
                        quantity_after: maximum,
                    },
                    receiver_position,
                    position,
                    source_after: remaining,
                });
            }
            if !free {
                return Err(Refusal::MainBackpackFull);
            }
            Ok(TransferPlan::NewEntry { position })
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use oteryn_gameplay_domain::domain::item_definition::ItemStackClass;

    fn id(seed: u8) -> [u8; 16] {
        [
            seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
        ]
    }

    fn definition(key: &str) -> TypedDefinitionRef {
        TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: "definition-r1".into(),
        }
    }

    fn facts(key: &str, stack: ItemStackClass) -> ItemDefinitionFacts {
        ItemDefinitionFacts {
            weight: Some(1),
            definition: definition(key),
            stack,
            container_capacity: None,
            container_slot_equip_pattern: false,
        }
    }

    fn backpack_facts(capacity: u32) -> ItemDefinitionFacts {
        ItemDefinitionFacts {
            weight: Some(1),
            definition: definition("fixture:b3.backpack"),
            stack: ItemStackClass::NonStackable,
            container_capacity: Some(capacity),
            container_slot_equip_pattern: true,
        }
    }

    const COIN: ItemStackClass = ItemStackClass::Stackable {
        proven_maximum: None,
    };

    fn item(seed: u8, key: &str, quantity: u32) -> InventoryItem {
        InventoryItem {
            item_instance_id: id(seed),
            definition: definition(key),
            quantity,
        }
    }

    fn entry(seed: u8, key: &str, quantity: u32, ordinal: u64) -> BackpackEntry {
        BackpackEntry {
            item: item(seed, key, quantity),
            placement_ordinal: ordinal,
        }
    }

    fn plan(
        source: &InventoryItem,
        stack: ItemStackClass,
        entries: &[BackpackEntry],
        capacity: u32,
    ) -> std::result::Result<TransferPlan, ItemTransferRefusal> {
        let slot = item(1, "fixture:b3.backpack", 1);
        let moved = facts(&source.definition.production_key, stack);
        let backpack = backpack_facts(capacity);
        plan_transfer(&PlanInput {
            destination: ItemTransferDestination::MainBackpack,
            item: &moved,
            backpack: Some(&backpack),
            source,
            source_has_entries: false,
            slot: Some(&slot),
            entries,
        })
    }

    fn at(ordinal: u64) -> ContainerEntryPosition {
        ContainerEntryPosition {
            parent_item_instance_id: id(1),
            placement_ordinal: ordinal,
        }
    }

    #[test]
    fn d83_shapes_pick_the_newest_compatible_stack_with_room() {
        let coin = |seed, quantity| item(seed, "fixture:b3.coin", quantity);
        // No compatible stack: a new entry with the next ordinal.
        assert_eq!(
            plan(&coin(9, 30), COIN, &[], 20),
            Ok(TransferPlan::NewEntry { position: at(1) })
        );
        // Full merge into the newest stack with room; quantities may differ.
        let entries = [
            entry(10, "fixture:b3.coin", 40, 1),
            entry(11, "fixture:b3.coin", 100, 3),
            entry(12, "fixture:b3.coin", 60, 2),
        ];
        assert_eq!(
            plan(&coin(9, 30), COIN, &entries, 20),
            Ok(TransferPlan::FullMerge {
                receiver: TransferReceiver {
                    item_instance_id: id(12),
                    quantity_before: 60,
                    quantity_after: 90,
                },
                receiver_position: at(2),
            })
        );
        // Top-up: the receiver reaches 100 and the remainder takes ordinal 4.
        assert_eq!(
            plan(&coin(9, 55), COIN, &entries, 20),
            Ok(TransferPlan::TopUp {
                receiver: TransferReceiver {
                    item_instance_id: id(12),
                    quantity_before: 60,
                    quantity_after: 100,
                },
                receiver_position: at(2),
                position: at(4),
                source_after: 15,
            })
        );
        // Exactly max: 60 + 40 = 100 is still a full merge.
        assert!(matches!(
            plan(&coin(9, 40), COIN, &entries, 20),
            Ok(TransferPlan::FullMerge { .. })
        ));
        // Top-up with no free entry is refused; a full merge still fits.
        assert_eq!(
            plan(&coin(9, 55), COIN, &entries, 3),
            Err(ItemTransferRefusal::MainBackpackFull)
        );
        assert!(matches!(
            plan(&coin(9, 30), COIN, &entries, 3),
            Ok(TransferPlan::FullMerge { .. })
        ));
        // Another revision is not compatible.
        let other_revision = [BackpackEntry {
            item: InventoryItem {
                definition: TypedDefinitionRef {
                    revision_ref: "definition-r2".into(),
                    ..definition("fixture:b3.coin")
                },
                ..coin(10, 10)
            },
            placement_ordinal: 5,
        }];
        assert_eq!(
            plan(&coin(9, 30), COIN, &other_revision, 20),
            Ok(TransferPlan::NewEntry { position: at(6) })
        );
        // A non-stackable item never merges.
        let stones = [entry(10, "fixture:b3.stone", 1, 1)];
        assert_eq!(
            plan(
                &item(9, "fixture:b3.stone", 1),
                ItemStackClass::NonStackable,
                &stones,
                20
            ),
            Ok(TransferPlan::NewEntry { position: at(2) })
        );
    }

    #[test]
    fn capacity_and_stack_ceilings_are_max_and_max_plus_one() {
        let full: Vec<BackpackEntry> = (1..=20)
            .map(|ordinal| entry(100 + ordinal, "fixture:b3.stone", 1, u64::from(ordinal)))
            .collect();
        let stone = item(9, "fixture:b3.stone", 1);
        // 19 entries: the 20th is placed; 20 entries: the 21st is refused.
        assert!(matches!(
            plan(&stone, ItemStackClass::NonStackable, &full[..19], 20),
            Ok(TransferPlan::NewEntry { .. })
        ));
        assert_eq!(
            plan(&stone, ItemStackClass::NonStackable, &full, 20),
            Err(ItemTransferRefusal::MainBackpackFull)
        );
        // Capacity 20 accepted, 21 refused.
        assert!(plan(&stone, ItemStackClass::NonStackable, &[], 20).is_ok());
        assert_eq!(
            plan(&stone, ItemStackClass::NonStackable, &[], 21),
            Err(ItemTransferRefusal::UnsupportedContainerCapacity)
        );
        // Stack ceiling: proven 100 accepted, 101 refused; quantity 100 / 101.
        let proven = |maximum| ItemStackClass::Stackable {
            proven_maximum: Some(maximum),
        };
        assert_eq!(stack_maximum(proven(100)), Ok((100, true)));
        assert_eq!(
            stack_maximum(proven(101)),
            Err(ItemTransferRefusal::UnsupportedStackMaximum)
        );
        assert_eq!(stack_maximum(COIN), Ok((100, true)));
        assert_eq!(
            stack_maximum(ItemStackClass::Unknown),
            Err(ItemTransferRefusal::UnknownStackClass)
        );
        let coin = |quantity| item(9, "fixture:b3.coin", quantity);
        assert!(plan(&coin(100), COIN, &[], 20).is_ok());
        assert_eq!(
            plan(&coin(101), COIN, &[], 20),
            Err(ItemTransferRefusal::QuantityAboveStackMaximum)
        );
        // A proven smaller maximum bounds the merge.
        let small = [entry(10, "fixture:b3.coin", 20, 1)];
        assert!(matches!(
            plan(&coin(10), proven(25), &small, 20),
            Ok(TransferPlan::TopUp {
                source_after: 5,
                ..
            })
        ));
        assert_eq!(GAMEITEM01_REACHABLE_ITEMS, 21);
        assert_eq!(GAMEITEM01_PLACEMENT_DEPTH, 1);
    }

    #[test]
    fn container_slot_and_backpack_refusals() {
        let bag = item(9, "fixture:b3.backpack", 1);
        let mut pattern = backpack_facts(20);
        let slot_plan =
            |facts: &ItemDefinitionFacts, slot: Option<&InventoryItem>, has_entries: bool| {
                plan_transfer(&PlanInput {
                    destination: ItemTransferDestination::ContainerSlot,
                    item: facts,
                    backpack: None,
                    source: &bag,
                    source_has_entries: has_entries,
                    slot,
                    entries: &[],
                })
            };
        assert_eq!(
            slot_plan(&pattern, None, false),
            Ok(TransferPlan::ContainerSlot)
        );
        let occupied = item(1, "fixture:b3.backpack", 1);
        assert_eq!(
            slot_plan(&pattern, Some(&occupied), false),
            Err(ItemTransferRefusal::ContainerSlotOccupied)
        );
        assert_eq!(
            slot_plan(&pattern, None, true),
            Err(ItemTransferRefusal::ContainerNotEmpty)
        );
        // A container-slot destination applies the same capacity bounds as
        // the main backpack: capacity 0 and 21 refused, 1 and 20 accepted.
        let zero_capacity = backpack_facts(0);
        assert_eq!(
            slot_plan(&zero_capacity, None, false),
            Err(ItemTransferRefusal::UnsupportedContainerCapacity)
        );
        let above_max_capacity = backpack_facts(21);
        assert_eq!(
            slot_plan(&above_max_capacity, None, false),
            Err(ItemTransferRefusal::UnsupportedContainerCapacity)
        );
        let min_capacity = backpack_facts(1);
        assert_eq!(
            slot_plan(&min_capacity, None, false),
            Ok(TransferPlan::ContainerSlot)
        );
        let max_capacity = backpack_facts(20);
        assert_eq!(
            slot_plan(&max_capacity, None, false),
            Ok(TransferPlan::ContainerSlot)
        );
        pattern.container_slot_equip_pattern = false;
        assert_eq!(
            slot_plan(&pattern, None, false),
            Err(ItemTransferRefusal::NotContainerSlotEquippable)
        );
        let stone = item(9, "fixture:b3.stone", 1);
        let stone_facts = facts("fixture:b3.stone", ItemStackClass::NonStackable);
        let backpack = backpack_facts(20);
        assert_eq!(
            plan_transfer(&PlanInput {
                destination: ItemTransferDestination::MainBackpack,
                item: &stone_facts,
                backpack: Some(&backpack),
                source: &stone,
                source_has_entries: false,
                slot: None,
                entries: &[],
            }),
            Err(ItemTransferRefusal::NoMainBackpack)
        );
        let wrong = facts("fixture:b3.other", ItemStackClass::NonStackable);
        assert_eq!(
            plan(&stone, ItemStackClass::Unknown, &[], 20),
            Err(ItemTransferRefusal::UnknownStackClass)
        );
        let slot = item(1, "fixture:b3.backpack", 1);
        assert_eq!(
            plan_transfer(&PlanInput {
                destination: ItemTransferDestination::MainBackpack,
                item: &wrong,
                backpack: Some(&backpack),
                source: &stone,
                source_has_entries: false,
                slot: Some(&slot),
                entries: &[],
            }),
            Err(ItemTransferRefusal::DefinitionMismatch)
        );
    }

    #[test]
    fn corpse_window_is_exclusive_to_the_top_damage_character_until_the_deadline() {
        let (owner, other) = (id(1), id(2));
        let materialized = 1_790_000_000_000_i64;
        let deadline = materialized + CORPSE_EXCLUSIVE_WINDOW_MS;
        assert_eq!(CORPSE_EXCLUSIVE_WINDOW_MS, 10_000);
        // The top-damage character may always take entries out.
        for now in [materialized, deadline - 1, deadline, deadline + 1] {
            assert!(corpse_window_allows(owner, owner, materialized, now));
        }
        // Anyone else: closed strictly before the deadline, open at it.
        assert!(!corpse_window_allows(
            other,
            owner,
            materialized,
            materialized
        ));
        assert!(!corpse_window_allows(
            other,
            owner,
            materialized,
            deadline - 1
        ));
        assert!(corpse_window_allows(other, owner, materialized, deadline));
        assert!(corpse_window_allows(
            other,
            owner,
            materialized,
            deadline + 1
        ));
        // An unrepresentable deadline never opens the window to others.
        assert!(!corpse_window_allows(other, owner, i64::MAX, i64::MAX));
    }
}
