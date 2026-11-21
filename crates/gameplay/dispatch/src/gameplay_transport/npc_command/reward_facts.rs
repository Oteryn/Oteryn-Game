//! Advisory source selection reuses the existing fenced, complete carried tree.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct RewardFacts {
    pub quantities: BTreeMap<String, u64>,
    pub level: u32,
    pub sinatuki_legacy_paid: bool,
}

pub(super) async fn read(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
    sinatuki_exchange: bool,
) -> Option<RewardFacts> {
    // Caller drops the ChannelRuntime guard before this owner read.
    let fence = command.item_fence?;
    if sinatuki_exchange
        && admission
            .root
            .has_sinatuki_legacy_payout(admission.character, fence.character_id)
            .await
            .ok()?
    {
        // Historical completion has no inventory or level requirement.
        return Some(RewardFacts {
            quantities: BTreeMap::new(),
            level: 0,
            sinatuki_legacy_paid: true,
        });
    }
    let inventory = admission
        .root
        .read_item_use_inventory(admission.character, admission.holder, fence)
        .await
        .ok()?;
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
            *quantity = quantity.checked_add(u64::from(item.quantity))?;
        }
    }
    // Level eligibility belongs to progression and does not require a vocation
    // or any spell-specific vitals. The quest writer rechecks it under Root.
    let progression = admission
        .root
        .read_character_progression(admission.character, fence.character_id)
        .await
        .ok()??;
    Some(RewardFacts {
        quantities,
        level: progression.level,
        sinatuki_legacy_paid: false,
    })
}
