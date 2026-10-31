//! Crystal Newhaven sealed doors: source-selected USE relocation, ordinary owner gates.
use super::world_map::{MapTargetRefusal, MapViewSource, resolve_map_target};
use crate::content::native_gameplay::NativeGameplayState;
use crate::foundation::MovementLocalPosition;
use crate::map::boot::BundleMap;
use crate::map::overlay::TilePos;
use crate::map::view;
use oteryn_protocol_oteryn::world_map::MapDefinition;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

pub(super) const REFUSAL: &str = "You need to be at least level 8.";
pub(super) const DESTINATION: MovementLocalPosition = MovementLocalPosition {
    x: 32555,
    y: 32497,
    floor: 7,
};

fn sealed_door(key: &str, pos: TilePos) -> bool {
    matches!(
        (key, pos.x, pos.y, pos.floor),
        (
            "oteryn:item.tibia.i51777" | "oteryn:item.tibia.i51778",
            32556,
            32497 | 32498,
            -7
        )
    )
}

/// Resolve the exact map entry, revision and source definition before applying the script.
/// `None` leaves every other item with the ordinary USE owner.
pub(super) fn resolve(
    map: &BundleMap,
    native: &NativeGameplayState,
    generation: [u8; 32],
    actor: TilePos,
    target: &WorldObjectTarget,
) -> Option<Result<(), UseDisposition>> {
    let source = MapViewSource {
        overlay: &map.overlay,
        facts: &map.facts,
        content_generation: generation,
        reset_epoch: 0,
    };
    let resolved = match resolve_map_target(&source, target, |_| None::<()>) {
        Ok(resolved) => resolved,
        Err(MapTargetRefusal::StateRevisionMismatch) => {
            return Some(Err(UseDisposition::StaleState));
        }
        Err(MapTargetRefusal::Use(disposition)) => return Some(Err(disposition)),
    };
    let tile = view::compose_tile(&map.overlay, &map.facts, resolved.pos).ok()??;
    let entry = tile
        .entries
        .iter()
        .find(|entry| entry.source.key() == view::MapEntryKey::Base(resolved.placement_key))?;
    let MapDefinition::Item(reference) = entry.facts.definition else {
        return None;
    };
    let index = native.item_index()?;
    let definition = super::item_look::typed_definition(index, reference)?;
    if !sealed_door(&definition.production_key, resolved.pos) {
        return None;
    }
    Some(if super::map_door::reachable(actor, resolved.pos) {
        Ok(())
    } else {
        Err(UseDisposition::TooFar)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newhaven_ferry_requires_the_exact_source_item_and_tile() {
        let first = TilePos {
            x: 32556,
            y: 32497,
            floor: -7,
        };
        let second = TilePos { y: 32498, ..first };
        assert!(sealed_door("oteryn:item.tibia.i51777", first));
        assert!(sealed_door("oteryn:item.tibia.i51778", second));
        assert!(sealed_door("oteryn:item.tibia.i51778", first));
        assert!(sealed_door("oteryn:item.tibia.i51777", second));
        assert!(!sealed_door(
            "oteryn:item.tibia.i51777",
            TilePos { floor: 7, ..first }
        ));
        assert!(!sealed_door(
            "oteryn:item.tibia.i51777",
            TilePos { x: 32557, ..first }
        ));
        assert_eq!(
            (DESTINATION.x, DESTINATION.y, DESTINATION.floor),
            (32555, 32497, 7)
        );
        assert_eq!(REFUSAL, "You need to be at least level 8.");
    }
}
