//! Advisory source selection reuses the existing fenced, complete carried tree.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::{NpcReadRefusal, npc_read_attempt, progression_read_error};
use crate::durability::item_use::ItemUseError;
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct RewardFacts {
    pub quantities: BTreeMap<String, u64>,
    pub level: u32,
}

pub(super) async fn read(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
) -> Result<RewardFacts, NpcReadRefusal> {
    // Caller drops the ChannelRuntime guard before this owner read.
    let fence = command.item_fence.ok_or(NpcReadRefusal::Stale)?;
    let budget = super::super::AdmissionRetryBudget::for_command(admission.root);
    let inventory = budget
        .retry_command_guard(async || {
            npc_read_attempt(
                admission
                    .root
                    .read_item_use_inventory(admission.character, admission.holder, fence)
                    .await,
                |error| match error {
                    ItemUseError::Unavailable(error) => Some(error),
                    _ => None,
                },
            )
        })
        .await
        .map_err(|error| NpcReadRefusal::from_durability(&error))?
        .map_err(|error| match error {
            ItemUseError::AuthorityRejected => NpcReadRefusal::Stale,
            _ => NpcReadRefusal::Unavailable,
        })?;
    let mut seen = BTreeSet::new();
    let mut quantities = BTreeMap::<String, u64>::new();
    for item in inventory
        .equipment
        .iter()
        .chain(inventory.backpack.iter().map(|bag| &bag.backpack))
        .chain(
            inventory
                .backpack
                .iter()
                .flat_map(|bag| bag.entries.iter().map(|entry| &entry.item)),
        )
        .chain(inventory.nested.iter().map(|(_, entry)| &entry.item))
    {
        if seen.insert(item.item_instance_id) && item.definition.family == "Item" {
            let quantity = quantities
                .entry(item.definition.production_key.clone())
                .or_default();
            *quantity = quantity
                .checked_add(u64::from(item.quantity))
                .ok_or(NpcReadRefusal::Unavailable)?;
        }
    }
    // Level eligibility belongs to progression and does not require a vocation
    // or any spell-specific vitals. The quest writer rechecks it under Root.
    let progression = budget
        .retry_command_guard(async || {
            npc_read_attempt(
                admission
                    .root
                    .read_character_progression(admission.character, fence.character_id)
                    .await,
                progression_read_error,
            )
        })
        .await
        .map_err(|error| NpcReadRefusal::from_durability(&error))?
        .map_err(|_| NpcReadRefusal::Unavailable)?
        .ok_or(NpcReadRefusal::Unavailable)?;
    Ok(RewardFacts {
        quantities,
        level: progression.level,
    })
}
