//! Depot uses the existing typed session exchange, including its inventory commit barrier.
pub use oteryn_session::{
    DepotAction, DepotEntry, DepotIntent, DepotOutcome, DepotReply, ItemEntry, ItemHandle,
};

pub use container_tree::ContainerViewOutcome;
pub use oteryn_session::{CharacterInventory, container_tree};
use oteryn_session::{Session, SessionError, SessionStream};

pub async fn send<S: SessionStream>(
    session: &mut Session<S>,
    intent: DepotIntent,
) -> Result<Option<DepotReply>, SessionError> {
    session.depot(intent).await
}

/// The same authoritative item projections used to resolve deposit handles on the server.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Items {
    pub inventory: CharacterInventory,
    pub views: Vec<ItemEntry>,
    pub containers_selected: bool,
}

pub fn items<S: SessionStream>(session: &Session<S>) -> Items {
    Items {
        containers_selected: session.is_selected(14),
        views: session
            .gated_snapshot(14)
            .and_then(|s| {
                container_tree::decode_container_views_with_house_capacity(
                    &s.payload,
                    session.is_selected(230),
                    session.is_selected(446),
                )
                .ok()
            })
            .map(|v| {
                carried_entries(
                    session
                        .inventory()
                        .unwrap_or(&CharacterInventory::default()),
                    &v,
                )
            })
            .unwrap_or_default(),
        inventory: session.inventory().cloned().unwrap_or_default(),
    }
}

/// Open a carried bag as a deposit source, reusing the server's container view route.
pub async fn open_container<S: SessionStream>(
    session: &mut Session<S>,
    handle: ItemHandle,
) -> Result<Option<ContainerViewOutcome>, SessionError> {
    session.open_depot_container(handle).await
}

// Admit only views connected to live carried handles. Parent links retain provenance
// when opening a nested bag replaces the parent's view. Ground views never seed it.
fn carried_entries(
    inventory: &CharacterInventory,
    views: &container_tree::ContainerViews,
) -> Vec<ItemEntry> {
    use std::collections::BTreeSet;
    let mut carried: BTreeSet<_> = inventory
        .main_backpack
        .iter()
        .chain(&inventory.entries)
        .chain(inventory.equipment.iter().map(|e| &e.item))
        .map(|e| e.handle)
        .collect();
    let mut admitted = BTreeSet::new();
    for _ in 0..views.views.len() {
        let before = admitted.len();
        for (index, view) in views.views.iter().enumerate() {
            if view.browse.is_none()
                && (carried.contains(&view.container_handle)
                    || view
                        .parent_handle
                        .is_some_and(|parent| carried.contains(&parent)))
            {
                admitted.insert(index);
                carried.insert(view.container_handle);
                carried.extend(view.entries.iter().map(|e| e.handle));
            }
        }
        if before == admitted.len() {
            break;
        }
    }
    views
        .views
        .iter()
        .enumerate()
        .filter(|(index, _)| admitted.contains(index))
        .flat_map(|(_, view)| view.entries.iter().copied())
        .collect()
}
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
