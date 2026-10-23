//! DEPOT-1 storage owner (migration 0151).
//!
//! A town's seventeen compartments contain the existing native ItemInstances;
//! neither lockers nor chests are mintable items. Inbox delivery continues to
//! use `character_inbox` and migration 0076, independently of depot capacity.
//!
//! Runtime may read these tables but has no placement/removal grants. Live
//! locker bindings, session-bound views, TRANSFER receipts and Inbox out-shapes
//! must land before a later migration enables a fenced writer. In particular,
//! these data values convey no current session, locker or Premium authority.

use super::item_transfer::InventoryItem;

/// Immediate durable custody. `town_id` preserves the map's u16 depot id,
/// including zero; World is supplied by the native item and Character root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterDepotEntry {
    pub item: InventoryItem,
    pub character_id: [u8; 16],
    pub world_id: [u8; 16],
    pub town_id: u16,
    pub box_number: u8,
    pub ordinal: u64,
}
