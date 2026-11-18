//! Crystal 201189f8 data-global/scripts/quests/targuna/creaturescripts_herald_of_fire.lua:51-59.
use crate::{
    foundation::MovementLocalPosition,
    map::{boot::BundleMap, overlay::TilePos},
};
use oteryn_gameplay_quests::quest::{source_deaths::DeathPortal, world_callbacks::Position};
use oteryn_gameplay_world::map::view::MapFacts;

pub(super) fn publish(
    map: &BundleMap,
    creature: &str,
    death: MovementLocalPosition,
    bindings: &[DeathPortal],
) {
    let (Ok(x), Ok(y), Ok(floor)) = (
        u16::try_from(death.x),
        u16::try_from(death.y),
        i8::try_from(-death.floor),
    ) else {
        return;
    };
    let death = Position { x, y, floor };
    for portal in bindings
        .iter()
        .filter(|portal| portal.admits(creature, death))
    {
        let tile = |p: Position| TilePos {
            x: p.x,
            y: p.y,
            floor: p.floor,
        };
        let at = tile(portal.position);
        let destination = tile(portal.destination);
        if [at, destination].into_iter().any(|pos| {
            map.facts.house_tile(pos) || map.overlay.base().tile(pos.x, pos.y, pos.floor).is_none()
        }) {
            continue;
        }
        let Ok(Some(current)) = crate::map::view::compose_tile(&map.overlay, &map.facts, at) else {
            continue;
        };
        if current
            .entries
            .iter()
            .any(|entry| entry.facts.appearance_id == portal.appearance)
        {
            continue;
        }
        map.facts
            .publish_quest_death_portal(at, destination, portal.appearance, portal.effect);
    }
}
