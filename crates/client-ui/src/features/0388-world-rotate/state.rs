use oteryn_client_runtime::inventory::{ItemEntry, ItemHandle};
use std::num::NonZeroU32;

pub type MapBinding = ([u8; 32], [u8; 32], u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProbeBinding {
    handle: ItemHandle,
    definition: NonZeroU32,
    inventory_generation: u64,
    map: MapBinding,
}

/// Only the selected inventory item's advisory Rotate affordance is retained.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InventoryProbe {
    selected: bool,
    inventory_generation: u64,
    pending: Option<ProbeBinding>,
    known: Option<(ProbeBinding, bool)>,
}

impl InventoryProbe {
    pub fn new(selected: bool) -> Self {
        Self {
            selected,
            ..Self::default()
        }
    }

    pub fn disable(&mut self) {
        self.selected = false;
        self.pending = None;
        self.known = None;
    }

    pub fn inventory_changed(&mut self) {
        self.known = None;
        if let Some(next) = self.inventory_generation.checked_add(1) {
            self.inventory_generation = next;
        } else {
            self.disable();
        }
        // Keep an in-flight binding until its response arrives. Reusing that slot
        // could otherwise associate an old response with a newer inventory query.
    }

    pub fn selection_changed(&mut self) {
        self.known = None;
    }

    fn binding(&self, item: ItemEntry, map: MapBinding) -> ProbeBinding {
        ProbeBinding {
            handle: item.handle,
            definition: item.item_definition_ref,
            inventory_generation: self.inventory_generation,
            map,
        }
    }

    pub(super) fn begin(&mut self, item: ItemEntry, map: MapBinding) -> bool {
        let binding = self.binding(item, map);
        if !self.selected
            || self.pending.is_some()
            || self.known.is_some_and(|(known, _)| known == binding)
        {
            return false;
        }
        self.pending = Some(binding);
        self.known = None;
        true
    }

    pub(super) fn finish(
        &mut self,
        item: Option<ItemEntry>,
        map: Option<MapBinding>,
        allowed: bool,
    ) {
        let Some(pending) = self.pending.take() else {
            return;
        };
        self.known = None;
        if !self.selected {
            return;
        }
        if let (Some(item), Some(map)) = (item, map)
            && self.binding(item, map) == pending
        {
            self.known = Some((pending, allowed));
        }
    }

    pub fn available(&self, item: ItemEntry, map: MapBinding) -> bool {
        self.selected
            && self.pending.is_none()
            && self.known == Some((self.binding(item, map), true))
    }
}
