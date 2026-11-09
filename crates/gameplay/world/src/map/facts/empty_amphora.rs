//! Qualification of one empty, flat, unbound native map amphora for a Ground push.
use super::{BundleFacts, Family, MapFacts, TilePos, pickup, placement_key};
use oteryn_gameplay_items::map_pickup::{PickupFacts, empty_amphora_push_quantity};

impl BundleFacts {
    /// The dispatcher supplies the current B3 tile depths and resolved native item key.
    /// Inventory pickup continues to use `pickup_entry`, which refuses this source.
    pub fn empty_amphora_push_entry(
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
        let found = self
            .placements
            .binary_search_by_key(&placement_key, |(key, _)| *key)
            .map_err(|_| missing)?;
        let placement = &self.placements[found].1;
        let palette = self
            .palette
            .get(placement.palette as usize)
            .ok_or(missing)?;
        if palette.family != Family::Item || palette.id != id || palette.facts.appearance_id != 2904
        {
            return Err(pickup::Ineligible::NotPickupable);
        }
        let attrs = &placement.attrs;
        let quantity = empty_amphora_push_quantity(
            key,
            PickupFacts {
                pickupable: palette.facts.pickupable,
                house: self.house_tile(pos),
                action: attrs.action.is_some(),
                unique: attrs.unique.is_some(),
                door: attrs.door.is_some(),
                depot: attrs.depot.is_some(),
                teleport: attrs.teleport.is_some(),
                dropped_teleport: placement.dropped,
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
            palette: placement.palette,
            quantity,
        })
    }
}
