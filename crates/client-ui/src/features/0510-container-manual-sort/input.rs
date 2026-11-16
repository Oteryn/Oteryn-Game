//! Tibia Global's Manual Sort Mode (capability 510 `CONTAINER_MANUAL_SORT_V1`): while it is on,
//! an item dropped on a container slot is placed in that slot instead of the first one.
use oteryn_client_runtime::inventory::ItemMoveDestination;

/// The mode of one play session: off when the session starts, and available only when the
/// session negotiated capability 510.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ManualSort {
    pub selected: bool,
    pub active: bool,
}

/// The mode of a newly admitted session: off, available when it selected capability 510.
#[must_use]
pub fn admitted(selected: bool) -> ManualSort {
    ManualSort {
        selected,
        active: false,
    }
}

impl ManualSort {
    /// The `ToggleManualSortMode` hotkey; `false` (and no change) without capability 510.
    pub fn toggle(&mut self) -> bool {
        if self.selected {
            self.active = !self.active;
        }
        self.selected
    }

    /// Container titles render orange while this is true.
    #[must_use]
    pub fn active(self) -> bool {
        self.selected && self.active
    }
}

/// The destination a drop sends: the container slot only while the mode is on, otherwise the
/// default placement.
#[must_use]
pub fn drop_destination(mode: ManualSort, destination: ItemMoveDestination) -> ItemMoveDestination {
    match destination {
        ItemMoveDestination::Container(handle, Some(_)) if !mode.active() => {
            ItemMoveDestination::Container(handle, None)
        }
        destination => destination,
    }
}

/// The destination of a drop on a list surface (the Windows-native container and inventory
/// windows): while the mode is on, the dropped-on `row` is the slot of the surface's container, and
/// a main backpack surface names the main backpack by its handle so that it can carry the slot.
/// Without the mode, a row or a known backpack, the surface's own destination is kept.
#[must_use]
pub fn row_destination(
    mode: ManualSort,
    destination: ItemMoveDestination,
    main_backpack: Option<oteryn_client_runtime::inventory::ItemHandle>,
    row: Option<u32>,
) -> ItemMoveDestination {
    match (destination, row) {
        (ItemMoveDestination::Container(handle, _), Some(row)) if mode.active() => {
            ItemMoveDestination::Container(handle, Some(row))
        }
        (ItemMoveDestination::MainBackpack, Some(row)) if mode.active() => main_backpack
            .map_or(destination, |backpack| {
                ItemMoveDestination::Container(backpack, Some(row))
            }),
        _ => destination,
    }
}

/// Tibia's orange container title while the mode is on (sRGB).
pub const ACTIVE_TITLE_RGB: [u8; 3] = [0xF7, 0x9C, 0x1F];

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
