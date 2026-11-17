//! Observer Eye's control chamber and west platform field.
//! Source placements: Crystal 201189f8, data-global/world/world.otbm, floor 14.

use super::*;

pub(super) fn mirrored_direction(
    from: MovementLocalPosition,
    to: MovementLocalPosition,
) -> Option<CardinalStep> {
    if from.floor != to.floor {
        return None;
    }
    use CardinalStep::*;
    match (to.x.checked_sub(from.x)?, to.y.checked_sub(from.y)?) {
        (0, -1) => Some(North),
        (1, -1) => Some(NorthEast),
        (1, 0) => Some(East),
        (1, 1) => Some(SouthEast),
        (0, 1) => Some(South),
        (-1, 1) => Some(SouthWest),
        (-1, 0) => Some(West),
        (-1, -1) => Some(NorthWest),
        _ => None,
    }
}

fn control_floor(cell: MovementLocalPosition) -> bool {
    cell.floor == 14 && (32781..=32783).contains(&cell.x) && (32912..=32914).contains(&cell.y)
}

fn platform_field(cell: MovementLocalPosition) -> bool {
    cell.floor == 14 && (32775..=32779).contains(&cell.x) && (32907..=32918).contains(&cell.y)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn mirror_step(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    cells: &NativeEntryMovementCells,
    controller: ExactActorRef,
    from: MovementLocalPosition,
    to: MovementLocalPosition,
    now_us: u64,
    blocking: &dyn crate::map::floor::CellBlocker,
) {
    if !control_floor(from) || !control_floor(to) {
        return;
    }
    let Some(direction) = mirrored_direction(from, to) else {
        return;
    };
    let Ok(census) = runtime.positioned_actor_census() else {
        return;
    };
    let mut partners = census.into_iter().filter_map(|(actor, snapshot, session)| {
        let session = session?;
        (actor != controller && platform_field(snapshot.position()) && !states.is_dead(actor))
            .then_some((actor, session, snapshot.position()))
    });
    let Some((partner, session, position)) = partners.next() else {
        return;
    };
    let (dx, dy) = direction.delta();
    let destination = MovementLocalPosition {
        x: position.x + dx,
        y: position.y + dy,
        ..position
    };
    if partners.next().is_some()
        || !platform_field(destination)
        || !matches!(runtime.position_occupied_by_other(partner, destination), Ok(false))
    {
        return;
    }
    // Use the usual collision and position owner. The inner turn has no
    // mirroring callback, so a reflected move cannot cause another reflection.
    let _ = step_in_channel_inner(
        runtime, states, cells, partner, session, now_us, direction, blocking, None, None,
    );
}
