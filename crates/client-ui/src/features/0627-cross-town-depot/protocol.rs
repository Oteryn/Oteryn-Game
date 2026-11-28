use super::{DepotIntent, DepotPanel, DepotRequest};

/// Only an explicit, negotiated town selection opts into cross-town routing.
pub fn request_for_selection(
    panel: &DepotPanel,
    intent: DepotIntent,
    slot: Option<u32>,
) -> DepotRequest {
    DepotRequest {
        cross_town: panel.cross_town_selected
            && panel.selected_town.is_some()
            && panel.selected_town == intent.town,
        intent,
        slot,
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
