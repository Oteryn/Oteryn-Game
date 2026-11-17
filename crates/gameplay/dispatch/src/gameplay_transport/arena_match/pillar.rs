//! Crystal 201189f svargrond_arena/creaturescripts_arena_kill.lua:22-35,
//! lib/quests/svargrond_arena.lua:207-246. Called with the native runtime guard
//! after current durable pit custody has been checked (or for its exact release).
use super::*;
use crate::map::facts::ArenaPillarProjection;

pub(super) fn project(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    state: &MatchState,
    projection: ArenaPillarProjection,
) -> bool {
    let Some(map) = owner.bundle_map() else {
        return true;
    };
    let Some(pit) = oteryn_gameplay_quests::quest::barbarian_arena::Pit::new(state.pit) else {
        return false;
    };
    let [x, y, floor] = pit.layout().pillar;
    let (Ok(x), Ok(y), Ok(floor)) = (u16::try_from(x), u16::try_from(y), i8::try_from(floor))
    else {
        return false;
    };
    let pos = crate::map::overlay::TilePos { x, y, floor };
    map.facts
        .project_svargrond_pillar(map.overlay.base(), pos, state.run, projection)
}
