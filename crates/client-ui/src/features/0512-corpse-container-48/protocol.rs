//! Open-corpse window bound for capability 512 (CORPSE48, owner 2026-11-17).
use oteryn_client_runtime::inventory::{
    MAX_CORPSE_CONTAINER_48_ENTRIES, MAX_OPEN_CONTAINER_ENTRIES,
};

/// Entries an open corpse window can show: 48 with capability 512, else the legacy 30.
pub const fn open_corpse_capacity(corpse_48_selected: bool) -> usize {
    if corpse_48_selected {
        MAX_CORPSE_CONTAINER_48_ENTRIES
    } else {
        MAX_OPEN_CONTAINER_ENTRIES
    }
}

#[cfg(test)]
#[test]
fn a_capability_512_corpse_window_holds_forty_eight_entries() {
    assert_eq!(open_corpse_capacity(true), 48);
    assert_eq!(open_corpse_capacity(false), 30);
}
