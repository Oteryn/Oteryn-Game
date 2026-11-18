//! Crystal 201189f8 globalevents_crate.lua:1-36. The Channel owner serializes each pass.
use crate::map::{
    boot::BundleMap,
    overlay::TilePos,
    view::{EntrySource, compose_tile},
};

const DETECTING: u16 = 11257;
const SAFE: u16 = 11258;
const DOORS: [(u16, u16); 7] = [
    (33385, 31139),
    (33385, 31134),
    (33385, 31126),
    (33385, 31119),
    (33385, 31118),
    (33380, 31093),
    (33380, 31085),
];

pub(super) fn tick(map: &BundleMap, now: u64) {
    let elapsed = map.facts.advance_wrath_trapdoor_cycle(now);
    if elapsed == 0 {
        return;
    }
    for (x, y) in DOORS {
        let at = TilePos { x, y, floor: -8 };
        if map.overlay.base().tile(x, y, at.floor).is_none() {
            continue;
        }
        let Ok(Some(tile)) = compose_tile(&map.overlay, &map.facts, at) else {
            continue;
        };
        let current = tile.entries.iter().find(|entry| {
            matches!(entry.facts.appearance_id, DETECTING | SAFE)
                && matches!(
                    entry.source,
                    EntrySource::Base { .. } | EntrySource::Added(_)
                )
        });
        match current {
            None => {
                let appearance = if elapsed.is_multiple_of(2) {
                    SAFE
                } else {
                    DETECTING
                };
                map.facts.create_movement_item(at, appearance, 0, || true);
            }
            Some(entry) if !elapsed.is_multiple_of(2) => {
                let key = match entry.source {
                    EntrySource::Base { placement_key, .. } => placement_key,
                    EntrySource::Added(id) => id.get(),
                    _ => continue,
                };
                let appearance = if entry.facts.appearance_id == DETECTING {
                    SAFE
                } else {
                    DETECTING
                };
                map.facts.transform_movement_item(key, appearance);
            }
            Some(_) => {}
        }
    }
}
