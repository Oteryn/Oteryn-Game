//! DEPOT0 §5 raw no-merge shape planning. These inputs are not current locker/view,
//! admitted content, session, item ownership or lock evidence. No function commits a move.
#![allow(
    dead_code,
    reason = "DEPOT0 shape planning; awaits its CP-composed depot TRANSFER wiring"
)]

use crate::domain::premium::PremiumGate;
use crate::durability::character_depot::{DepotBox, insertion_fits, next_ordinal};
use crate::durability::item_transfer::{
    BackpackEntry, ContainerEntryPosition, GAMEITEM01_CONTAINER_ENTRIES_MAX, InventoryItem,
    ItemDefinitionFacts, ItemTransferDestination, ItemTransferRefusal, PlanInput, TransferPlan,
    plan_transfer, stack_maximum,
};
use std::collections::BTreeSet;
use std::num::NonZeroU64;

/// The two accepted transfer shapes; no protocol/audit shape numbers are assigned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DepotShape {
    BackpackEntryToBox {
        box_id: DepotBox,
        ordinal: NonZeroU64,
    },
    BoxToBackpack {
        destination: WithdrawDestination,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WithdrawDestination {
    ContainerSlot,
    DirectEntry(ContainerEntryPosition),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DepotRefusal {
    InvalidSnapshot,
    UnsupportedSource,
    NoRoom,
    Native(ItemTransferRefusal),
}

/// Raw source classification only. Future command admission must independently resolve it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DepositSource {
    MainBackpackDirectEntry,
    Ground,
    CorpseEntry,
    Other,
}

/// Capacity/count and content/contents are raw facts, not authority. Each accepted plan remains
/// one item / two location lines / three work units, with the complete item identity retained.
pub(crate) struct DepositInput<'a> {
    pub(crate) source: DepositSource,
    pub(crate) item: &'a InventoryItem,
    pub(crate) facts: &'a ItemDefinitionFacts,
    pub(crate) contents: &'a [BackpackEntry],
    pub(crate) box_id: DepotBox,
    pub(crate) entries: u32,
    pub(crate) highest: u64,
    pub(crate) premium: PremiumGate,
}

pub(crate) fn plan_deposit(input: &DepositInput<'_>) -> Result<DepotShape, DepotRefusal> {
    if input.source != DepositSource::MainBackpackDirectEntry {
        return Err(DepotRefusal::UnsupportedSource);
    }
    check_item(input.item, input.facts, input.contents)?;
    if !insertion_fits(input.entries, input.premium) {
        return Err(DepotRefusal::NoRoom);
    }
    let ordinal = next_ordinal(input.highest).ok_or(DepotRefusal::NoRoom)?;
    Ok(DepotShape::BackpackEntryToBox {
        box_id: input.box_id,
        ordinal,
    })
}

fn check_item(
    item: &InventoryItem,
    facts: &ItemDefinitionFacts,
    contents: &[BackpackEntry],
) -> Result<(), DepotRefusal> {
    if item.definition != facts.definition || item.quantity == 0 {
        return Err(DepotRefusal::InvalidSnapshot);
    }
    let (maximum, _) = stack_maximum(facts.stack).map_err(DepotRefusal::Native)?;
    if item.quantity > maximum {
        return Err(DepotRefusal::Native(
            ItemTransferRefusal::QuantityAboveStackMaximum,
        ));
    }
    if !contents.is_empty() {
        return Err(DepotRefusal::Native(ItemTransferRefusal::ContainerNotEmpty));
    }
    Ok(())
}

/// Unlike pickup, DEPOT0 forbids FullMerge/TopUp. The native slot branch is reused as-is;
/// the direct-entry branch counts every row, keeps full quantity and chooses highest+1.
pub(crate) fn plan_withdraw(input: &PlanInput<'_>) -> Result<DepotShape, DepotRefusal> {
    check_item(input.source, input.item, &[])?;
    if input.source_has_entries {
        return Err(DepotRefusal::Native(ItemTransferRefusal::ContainerNotEmpty));
    }
    if input.slot.is_none() {
        if input.backpack.is_some() || !input.entries.is_empty() {
            return Err(DepotRefusal::InvalidSnapshot);
        }
        let native = plan_transfer(&PlanInput {
            destination: ItemTransferDestination::ContainerSlot,
            ..*input
        })
        .map_err(DepotRefusal::Native)?;
        return match native {
            TransferPlan::ContainerSlot => Ok(DepotShape::BoxToBackpack {
                destination: WithdrawDestination::ContainerSlot,
            }),
            _ => Err(DepotRefusal::InvalidSnapshot),
        };
    }
    let slot = input.slot.ok_or(DepotRefusal::InvalidSnapshot)?;
    let backpack = input.backpack.ok_or(DepotRefusal::InvalidSnapshot)?;
    if slot.item_instance_id == input.source.item_instance_id
        || slot.definition != backpack.definition
        || slot.quantity != 1
    {
        return Err(DepotRefusal::InvalidSnapshot);
    }
    let capacity = backpack
        .container_capacity
        .filter(|n| (1..=GAMEITEM01_CONTAINER_ENTRIES_MAX).contains(n))
        .ok_or(DepotRefusal::Native(
            ItemTransferRefusal::UnsupportedContainerCapacity,
        ))?;
    if input.entries.len() >= capacity as usize {
        return Err(DepotRefusal::NoRoom);
    }
    let mut ids = BTreeSet::new();
    let mut ordinals = BTreeSet::new();
    for entry in input.entries {
        if entry.item.quantity == 0
            || entry.placement_ordinal == 0
            || entry.item.item_instance_id == slot.item_instance_id
            || entry.item.item_instance_id == input.source.item_instance_id
            || !ids.insert(entry.item.item_instance_id)
            || !ordinals.insert(entry.placement_ordinal)
        {
            return Err(DepotRefusal::InvalidSnapshot);
        }
    }
    let highest = ordinals.last().copied().unwrap_or(0);
    let ordinal = next_ordinal(highest).ok_or(DepotRefusal::NoRoom)?;
    Ok(DepotShape::BoxToBackpack {
        destination: WithdrawDestination::DirectEntry(ContainerEntryPosition {
            parent_item_instance_id: slot.item_instance_id,
            placement_ordinal: ordinal.get(),
        }),
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::durability::item_mint::TypedDefinitionRef;
    use crate::durability::item_transfer::ItemStackClass;

    fn item(id: u8, key: &str, quantity: u32) -> InventoryItem {
        let mut bytes = [0; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = id;
        InventoryItem {
            item_instance_id: bytes,
            quantity,
            definition: TypedDefinitionRef {
                family: "Item".into(),
                production_key: key.into(),
                revision_ref: "r1".into(),
            },
        }
    }
    fn facts(item: &InventoryItem) -> ItemDefinitionFacts {
        ItemDefinitionFacts {
            weight: None,
            definition: item.definition.clone(),
            stack: ItemStackClass::Stackable {
                proven_maximum: None,
            },
            container_capacity: None,
            container_slot_equip_pattern: false,
        }
    }
    fn bag(item: &InventoryItem, capacity: u32) -> ItemDefinitionFacts {
        ItemDefinitionFacts {
            stack: ItemStackClass::NonStackable,
            container_capacity: Some(capacity),
            container_slot_equip_pattern: true,
            ..facts(item)
        }
    }
    #[test]
    fn depot_deposit_whole_stack_counts_once_and_rejects_unsupported_sources_or_contents() {
        let moved = item(1, "fixture:coin", 100);
        let facts = facts(&moved);
        let mut input = DepositInput {
            source: DepositSource::MainBackpackDirectEntry,
            item: &moved,
            facts: &facts,
            contents: &[],
            box_id: DepotBox::new(17).expect("box"),
            entries: 1999,
            highest: 90,
            premium: PremiumGate::CLOSED,
        };
        assert_eq!(
            plan_deposit(&input),
            Ok(DepotShape::BackpackEntryToBox {
                box_id: input.box_id,
                ordinal: NonZeroU64::new(91).expect("ordinal")
            })
        );
        for source in [
            DepositSource::Ground,
            DepositSource::CorpseEntry,
            DepositSource::Other,
        ] {
            input.source = source;
            assert_eq!(plan_deposit(&input), Err(DepotRefusal::UnsupportedSource));
        }
        input.source = DepositSource::MainBackpackDirectEntry;
        input.entries = 2000;
        assert_eq!(plan_deposit(&input), Err(DepotRefusal::NoRoom));
        input.entries = 1999;
        input.highest = u64::MAX;
        assert_eq!(plan_deposit(&input), Err(DepotRefusal::NoRoom));
        input.highest = 90;
        let contents = [BackpackEntry {
            item: item(2, "fixture:child", 1),
            placement_ordinal: 1,
        }];
        input.contents = &contents;
        assert_eq!(
            plan_deposit(&input),
            Err(DepotRefusal::Native(ItemTransferRefusal::ContainerNotEmpty))
        );
    }
    #[test]
    fn depot_withdraw_uses_native_container_pattern_when_slot_is_empty() {
        let moved = item(1, "fixture:bag", 1);
        let facts = bag(&moved, 20);
        let mut input = PlanInput {
            destination: ItemTransferDestination::MainBackpack,
            item: &facts,
            backpack: None,
            source: &moved,
            source_has_entries: false,
            slot: None,
            entries: &[],
        };
        assert_eq!(
            plan_withdraw(&input),
            Ok(DepotShape::BoxToBackpack {
                destination: WithdrawDestination::ContainerSlot
            })
        );
        // Definition facts for an equipped backpack cannot exist in an empty-slot snapshot.
        // Reject this before the native ContainerSlot planner, which never uses backpack facts.
        input.backpack = Some(&facts);
        assert_eq!(plan_withdraw(&input), Err(DepotRefusal::InvalidSnapshot));
        input.backpack = None;
        input.source_has_entries = true;
        assert_eq!(
            plan_withdraw(&input),
            Err(DepotRefusal::Native(ItemTransferRefusal::ContainerNotEmpty))
        );
        let mut missing_pattern = facts.clone();
        missing_pattern.container_slot_equip_pattern = false;
        input = PlanInput {
            item: &missing_pattern,
            source_has_entries: false,
            ..input
        };
        assert_eq!(
            plan_withdraw(&input),
            Err(DepotRefusal::Native(
                ItemTransferRefusal::NotContainerSlotEquippable
            ))
        );
    }
    #[test]
    fn depot_withdraw_never_merges_and_full_backpack_refuses_even_compatible_stack() {
        let moved = item(1, "fixture:coin", 10);
        let facts = facts(&moved);
        let slot = item(2, "fixture:bag", 1);
        let backpack = bag(&slot, 2);
        let entries = [BackpackEntry {
            item: item(3, "fixture:coin", 90),
            placement_ordinal: 7,
        }];
        let input = PlanInput {
            destination: ItemTransferDestination::MainBackpack,
            item: &facts,
            backpack: Some(&backpack),
            source: &moved,
            source_has_entries: false,
            slot: Some(&slot),
            entries: &entries,
        };
        assert!(
            matches!(plan_transfer(&input), Ok(TransferPlan::FullMerge { .. })),
            "native pickup positive control"
        );
        assert_eq!(
            plan_withdraw(&input),
            Ok(DepotShape::BoxToBackpack {
                destination: WithdrawDestination::DirectEntry(ContainerEntryPosition {
                    parent_item_instance_id: slot.item_instance_id,
                    placement_ordinal: 8
                })
            })
        );
        let full = bag(&slot, 1);
        let input = PlanInput {
            backpack: Some(&full),
            ..input
        };
        assert_eq!(plan_withdraw(&input), Err(DepotRefusal::NoRoom));
    }
    #[test]
    fn depot_withdraw_rejects_unknown_facts_corrupt_rows_and_ordinal_overflow() {
        let moved = item(1, "fixture:coin", 10);
        let mut facts = facts(&moved);
        let slot = item(2, "fixture:bag", 1);
        let backpack = bag(&slot, 3);
        let mut entries = vec![BackpackEntry {
            item: item(3, "fixture:coin", 90),
            placement_ordinal: 7,
        }];
        let run = |facts: &ItemDefinitionFacts, entries: &[BackpackEntry]| {
            plan_withdraw(&PlanInput {
                destination: ItemTransferDestination::MainBackpack,
                item: facts,
                backpack: Some(&backpack),
                source: &moved,
                source_has_entries: false,
                slot: Some(&slot),
                entries,
            })
        };
        facts.stack = ItemStackClass::Unknown;
        assert_eq!(
            run(&facts, &entries),
            Err(DepotRefusal::Native(ItemTransferRefusal::UnknownStackClass))
        );
        facts.stack = ItemStackClass::Stackable {
            proven_maximum: None,
        };
        entries[0].placement_ordinal = u64::MAX;
        assert_eq!(run(&facts, &entries), Err(DepotRefusal::NoRoom));
        entries[0].placement_ordinal = 0;
        assert_eq!(run(&facts, &entries), Err(DepotRefusal::InvalidSnapshot));
        entries[0].placement_ordinal = 7;
        entries.push(entries[0].clone());
        assert_eq!(run(&facts, &entries), Err(DepotRefusal::InvalidSnapshot));
        entries.pop();
        facts.definition.revision_ref = "different".into();
        assert_eq!(run(&facts, &entries), Err(DepotRefusal::InvalidSnapshot));
    }
}
