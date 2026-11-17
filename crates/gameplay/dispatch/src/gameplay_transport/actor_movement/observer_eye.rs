//! Observer Eye's control chamber and west platform field.
//! Source placements: Crystal 201189f8, data-global/world/world.otbm, floor 14.

use super::{
    CardinalStep, ChannelRuntimeV1, ChannelSpellStates, ExactActorRef, GameSessionId,
    MovementLocalPosition, NativeEntryMovementCells,
};

#[path = "observer_eye/bindings.rs"]
mod bindings;

pub(super) fn mirrored_direction(
    from: MovementLocalPosition,
    to: MovementLocalPosition,
) -> Option<CardinalStep> {
    if !on_tile(from, bindings::TOPAZ) || !on_ornament(to) {
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

const ATTEMPT_MICROS: u64 = 300_000_000;

type Player = (ExactActorRef, GameSessionId);

#[derive(Debug, Clone, Copy)]
pub(in crate::gameplay_transport) struct ObserverEyePair {
    controller: Player,
    partner: Player,
    expires: u64,
}

fn on_tile(cell: MovementLocalPosition, tile: (i32, i32)) -> bool {
    cell.floor == bindings::FLOOR && (cell.x, cell.y) == tile
}

fn on_ornament(cell: MovementLocalPosition) -> bool {
    cell.floor == bindings::FLOOR && bindings::CONTROLS.binary_search(&(cell.x, cell.y)).is_ok()
}

fn platform_field(cell: MovementLocalPosition) -> bool {
    cell.floor == bindings::FLOOR
        && bindings::WEST_FIELD
            .binary_search(&(cell.x, cell.y))
            .is_ok()
}

fn current_player(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    (actor, session): Player,
) -> Option<MovementLocalPosition> {
    let (current, position) = runtime.positioned_player_for_session(session).ok()??;
    (current == actor && !states.is_dead(actor)).then_some(position.position())
}

fn current_pair(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    now_us: u64,
) -> Option<ObserverEyePair> {
    let pair = states.observer_eye_pair?;
    if now_us >= pair.expires {
        return None;
    }
    let controller = current_player(runtime, states, pair.controller)?;
    let partner = current_player(runtime, states, pair.partner)?;
    ((on_tile(controller, bindings::TOPAZ) || on_ornament(controller)) && platform_field(partner))
        .then_some(pair)
}

pub(super) fn holds_partner(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    actor: ExactActorRef,
    now_us: u64,
) -> bool {
    states
        .observer_eye_pair
        .is_some_and(|pair| pair.partner.0 == actor)
        && current_pair(runtime, states, now_us).is_some()
}

fn pair_on_pads(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    now_us: u64,
) -> Option<ObserverEyePair> {
    let players = runtime.positioned_actor_census().ok()?;
    let player_on_pad = |pad| {
        let mut matches = players.iter().filter_map(|&(actor, snapshot, session)| {
            let player = (actor, session?);
            (on_tile(snapshot.position(), pad) && current_player(runtime, states, player).is_some())
                .then_some(player)
        });
        let player = matches.next()?;
        matches.next().is_none().then_some(player)
    };
    Some(ObserverEyePair {
        controller: player_on_pad(bindings::TOPAZ)?,
        partner: player_on_pad(bindings::EMERALD)?,
        expires: now_us.checked_add(ATTEMPT_MICROS)?,
    })
}

#[allow(
    clippy::too_many_arguments,
    reason = "the reflected step combines the exact controller move and already qualified map facts"
)]
pub(super) fn mirror_step(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    cells: &NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    from: MovementLocalPosition,
    to: MovementLocalPosition,
    now_us: u64,
    blocking: &dyn crate::map::floor::CellBlocker,
) {
    let arrived_on_pad = on_tile(to, bindings::TOPAZ) || on_tile(to, bindings::EMERALD);
    if !arrived_on_pad
        && !states
            .observer_eye_pair
            .is_some_and(|pair| [pair.controller.0, pair.partner.0].contains(&actor))
    {
        return;
    }
    let Some(pair) = current_pair(runtime, states, now_us) else {
        states.observer_eye_pair = if arrived_on_pad && crate::map::floor::table_of(cells).is_some()
        {
            pair_on_pads(runtime, states, now_us)
        } else {
            None
        };
        return;
    };
    // Each ornament issues a command from the central topaz. Returning to
    // the topaz lets the controller give the same command again.
    if pair.controller != (actor, session) {
        return;
    }
    let Some(direction) = mirrored_direction(from, to) else {
        return;
    };
    let Some(position) = current_player(runtime, states, pair.partner) else {
        return;
    };
    let (partner, partner_session) = pair.partner;
    let (dx, dy) = direction.delta();
    let destination = MovementLocalPosition {
        x: position.x + dx,
        y: position.y + dy,
        ..position
    };
    if !platform_field(destination)
        || !matches!(
            runtime.position_occupied_by_other(partner, destination),
            Ok(false)
        )
    {
        return;
    }
    // Keep the reflected move in this field even if a later map revision
    // adds a floor change or teleport to one of its cells.
    let Ok(expected) = runtime.read_actor_position(partner) else {
        return;
    };
    let Ok(proof) = crate::map::floor::prepare_bundle_step(
        runtime,
        cells,
        partner,
        partner_session,
        expected,
        direction,
        false,
        blocking,
    ) else {
        return;
    };
    if proof.destination() == destination {
        let _ = runtime.commit_source_step(proof);
    }
}
