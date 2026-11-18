//! Crystal 201189f8, dangerous_depth/creaturescripts_snail_slime_kill.lua.
use super::ComposedFreshAdmission;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition, RuntimeScopeRefV1};

const COUNT: &str = "oteryn:creature.the_count_of_the_core";
const SLIME: &str = "oteryn:creature.snail_slime";
const SPECTATOR_RANGE: u32 = 20;
const SLIME_LIMIT: usize = 3;

pub(super) fn tick(owner: &ComposedFreshAdmission<'_, '_, '_>, runtime: &mut ChannelRuntimeV1) {
    let Ok(census) = runtime.current_live_creature_census() else {
        return;
    };
    let Some(count) = census.iter().find_map(|actor| {
        if !matches!(actor.runtime_scope(), RuntimeScopeRefV1::Instance { .. }) {
            return None;
        }
        let snapshot = runtime.companion_snapshot(*actor).ok()?;
        (snapshot.state.policy.definition_key == COUNT
            && snapshot.health > 0
            && snapshot.health <= snapshot.maximum_health / 2)
            .then_some(snapshot.position)
    }) else {
        return;
    };
    let nearby = |cell: MovementLocalPosition| {
        cell.floor == count.floor
            && cell.x.abs_diff(count.x) <= SPECTATOR_RANGE
            && cell.y.abs_diff(count.y) <= SPECTATOR_RANGE
    };
    let slimes = census.iter().filter(|actor| {
        runtime.companion_snapshot(**actor).is_ok_and(|snapshot| {
            snapshot.state.policy.definition_key == SLIME && nearby(snapshot.position)
        })
    });
    if slimes.take(SLIME_LIMIT).count() == SLIME_LIMIT {
        return;
    }

    // The source requests extended placement starting at the Count's occupied cell.
    for (dx, dy) in [
        (0, 0),
        (0, -1),
        (1, 0),
        (0, 1),
        (-1, 0),
        (1, -1),
        (1, 1),
        (-1, 1),
        (-1, -1),
    ] {
        if let (Some(x), Some(y)) = (count.x.checked_add(dx), count.y.checked_add(dy))
            && super::primal_menace::spawn_creature(
                owner,
                runtime,
                SLIME,
                MovementLocalPosition { x, y, ..count },
            )
            .is_some()
        {
            break;
        }
    }
}
