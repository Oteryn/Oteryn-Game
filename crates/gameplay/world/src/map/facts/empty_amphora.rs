//! Qualification of empty, flat, unbound native map vessels for a Ground push.
use super::{BundleFacts, Family, MapDefinition, MapFacts, TilePos, pickup, placement_key};
use oteryn_gameplay_items::map_pickup::{
    PickupFacts, empty_amphora_push_quantity, empty_barrel_push_quantity,
};
use std::num::NonZeroU32;

impl BundleFacts {
    /// Map projection reuses the move owner's exact immutable source qualification.
    pub(super) fn empty_vessel_pushable(
        &self,
        pos: TilePos,
        ordinal: u8,
        id: u32,
        depths: &[u8],
        definition: NonZeroU32,
    ) -> bool {
        ["oteryn:item.tibia.i2904", "oteryn:item.tibia.i2523"]
            .into_iter()
            .any(|key| {
                self.empty_vessel_push_entry(pos, ordinal, id, depths, key)
                    .is_ok_and(|entry| {
                        self.palette
                            .get(entry.palette as usize)
                            .is_some_and(|palette| {
                                palette.facts.definition == MapDefinition::Item(definition)
                            })
                    })
            })
    }

    /// The dispatcher supplies the current B3 tile depths and resolved native item key.
    /// Inventory pickup continues to use `pickup_entry`, which refuses this source.
    pub fn empty_vessel_push_entry(
        &self,
        pos: TilePos,
        ordinal: u8,
        id: u32,
        depths: &[u8],
        key: &str,
    ) -> Result<pickup::EligibleEntry, pickup::Ineligible> {
        let missing = pickup::Ineligible::NotTopLevel;
        if depths.is_empty() || depths.len() > 64 || usize::from(ordinal) >= depths.len() {
            return Err(missing);
        }
        if depths.iter().any(|depth| *depth != 0) {
            return Err(pickup::Ineligible::Contents);
        }
        let placement_key = placement_key(pos, ordinal).ok_or(missing)?;
        let placement = self
            .placements
            .binary_search_by_key(&placement_key, |(key, _)| *key)
            .ok()
            .map(|found| &self.placements[found].1);
        // B3 omits plain placements from the sparse attribute index. Fall back
        // to the unique palette entry, just as the base map view does.
        let palette_index = placement
            .map(|placement| placement.palette)
            .or_else(|| self.unique_index(id))
            .ok_or(missing)?;
        let palette = self.palette.get(palette_index as usize).ok_or(missing)?;
        if palette.family != Family::Item || palette.id != id {
            return Err(pickup::Ineligible::NotPickupable);
        }
        let default_attrs = Default::default();
        let attrs = placement.map_or(&default_attrs, |placement| &placement.attrs);
        let qualify = match palette.facts.appearance_id {
            2523 => empty_barrel_push_quantity,
            2904 => empty_amphora_push_quantity,
            _ => return Err(pickup::Ineligible::NotPickupable),
        };
        let quantity = qualify(
            key,
            PickupFacts {
                pickupable: palette.facts.pickupable,
                house: self.house_tile(pos),
                action: attrs.action.is_some(),
                unique: attrs.unique.is_some(),
                door: attrs.door.is_some(),
                depot: attrs.depot.is_some(),
                teleport: attrs.teleport.is_some(),
                dropped_teleport: placement.is_some_and(|placement| placement.dropped),
                contents: false,
                text: attrs.text.is_some(),
                description: attrs.description.is_some(),
                charges: attrs.charges.is_some(),
                quantity: attrs.count.map_or(1, u32::from),
            },
        )?;
        Ok(pickup::EligibleEntry {
            placement_key,
            origin: pos,
            ordinal,
            palette: palette_index,
            quantity,
        })
    }
}
