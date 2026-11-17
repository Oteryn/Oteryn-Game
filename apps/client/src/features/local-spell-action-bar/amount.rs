//! Counts only server-visible inventory entries; unopened storage is never inferred.
use crate::play::mailbox::Mailbox;
use std::collections::BTreeSet;
pub(super) fn visible_count(mailbox: &Mailbox, definition: u32) -> Option<u64> {
    let inventory = mailbox.inventory.inventory.as_ref()?;
    let mut seen = BTreeSet::new();
    let carried = inventory
        .entries
        .iter()
        .chain(inventory.main_backpack.iter())
        .chain(inventory.equipment.iter().map(|entry| &entry.item));
    let containers = mailbox
        .containers
        .views
        .iter()
        .flat_map(|views| &views.views)
        .flat_map(|view| &view.entries);
    Some(
        carried
            .chain(containers)
            .filter(|item| item.item_definition_ref.get() == definition && seen.insert(item.handle))
            .map(|item| u64::from(item.count.get()))
            .sum(),
    )
}
