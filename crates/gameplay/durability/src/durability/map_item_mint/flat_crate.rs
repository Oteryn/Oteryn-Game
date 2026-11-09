//! One empty crate on a flat immutable map tile; no nested-map expansion.
use super::super::item_transfer::ItemStackClass;
use super::{ItemDefinitionFacts, MapItemPlacement};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlatMapCrateProof {
    bundle: [u8; 32],
    placement: u64,
}
fn native_crate(item: &ItemDefinitionFacts) -> bool {
    item.definition.family == "Item"
        && item.definition.production_key == "oteryn:item.tibia.i2471"
        && item.definition.revision_ref == "definition-r1"
        && item.container_capacity == Some(15)
        && !item.container_slot_equip_pattern
        && item.stack == ItemStackClass::NonStackable
}
impl FlatMapCrateProof {
    /// Depths come from the current qualified B3 tile, never client data.
    /// Requiring a flat tile conservatively excludes every source child entry.
    pub fn qualify(
        item: &ItemDefinitionFacts,
        bundle: [u8; 32],
        placement: u64,
        depths: &[u8],
    ) -> Option<Self> {
        let at = MapItemPlacement::of_key(placement)?;
        (native_crate(item)
            && !depths.is_empty()
            && depths.len() <= 64
            && usize::from(at.ordinal) < depths.len()
            && depths.iter().all(|depth| *depth == 0))
        .then_some(Self { bundle, placement })
    }
    pub(super) fn matches(
        &self,
        item: &ItemDefinitionFacts,
        bundle: [u8; 32],
        placement: u64,
    ) -> bool {
        native_crate(item) && self.bundle == bundle && self.placement == placement
    }
    pub(super) fn append_binding(&self, bytes: &mut Vec<u8>) {
        bytes.extend_from_slice(b"oteryn:flat-map-crate/v1:");
        bytes.extend_from_slice(&self.bundle);
        bytes.extend_from_slice(&self.placement.to_be_bytes());
    }
}
