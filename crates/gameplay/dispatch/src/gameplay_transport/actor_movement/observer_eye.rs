//! Observer Eye's control chamber and west platform field.
//! Source placements: Crystal 201189f8, data-global/world/world.otbm, floor 14.

use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
use crate::map::boot::BundleCollisionIndexExt;

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
const PLATFORM_INTERVAL_MICROS: u64 = 2_000_000;
// These are the four platform appearances qualified by the shipped World.
const PLATFORM_PHASES: [u16; 4] = [24912, 24889, 24898, 24899];
const WEST_EXIT: MovementLocalPosition = MovementLocalPosition {
    x: 32777,
    y: 32904,
    floor: bindings::FLOOR,
};

const EAST_EXIT: MovementLocalPosition = MovementLocalPosition {
    x: 32784,
    ..WEST_EXIT
};

fn is_submerged(appearance: u16) -> bool {
    matches!(appearance, 24891..=24893 | 24912)
}

type Player = (ExactActorRef, GameSessionId);

#[derive(Debug, Clone, Copy)]
pub(in crate::gameplay_transport) struct ObserverEyePair {
    controller: Player,
    partner: Player,
    expires: u64,
    next_platform_cycle: u64,
}

fn on_tile(cell: MovementLocalPosition, tile: (i32, i32)) -> bool {
    cell.floor == bindings::FLOOR && (cell.x, cell.y) == tile
}

fn on_ornament(cell: MovementLocalPosition) -> bool {
    cell.floor == bindings::FLOOR && bindings::CONTROLS.binary_search(&(cell.x, cell.y)).is_ok()
}

fn platform_at(map: &crate::map::boot::BundleMap, at: MovementLocalPosition) -> Option<(u64, u16)> {
    let pos = crate::gameplay_transport::soul_war_scene::wire_position(at)?;
    let tile = crate::map::view::compose_tile(&map.overlay, &map.facts, pos).ok()??;
    tile.entries.into_iter().find_map(|entry| {
        let crate::map::view::EntrySource::Base { placement_key, .. } = entry.source else {
            return None;
        };
        let appearance = entry.facts.appearance_id;
        (PLATFORM_PHASES.contains(&appearance) || is_submerged(appearance))
            .then_some((placement_key, appearance))
    })
}

impl ChannelSpellStates {
    pub(in crate::gameplay_transport) fn next_observer_eye_due(&self) -> Option<u64> {
        self.observer_eye_pair
            .map(|pair| pair.expires.min(pair.next_platform_cycle))
    }
}

fn eject_expired_pair(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    cells: &NativeEntryMovementCells,
    map: &crate::map::boot::BundleMap,
    pair: ObserverEyePair,
) {
    let closed = crate::map::floor::BundleDoors(&map.facts);
    let mut exits = Vec::with_capacity(2);
    for ((actor, session), destination) in [(pair.controller, EAST_EXIT), (pair.partner, WEST_EXIT)]
    {
        let Ok(expected) = runtime.read_actor_position(actor) else {
            continue;
        };
        let at = expected.position();
        let in_chamber = if destination == EAST_EXIT {
            on_tile(at, bindings::TOPAZ) || on_ornament(at)
        } else {
            platform_field(at)
        };
        if !in_chamber || states.is_dead(actor) {
            continue;
        }
        let Ok(proof) = crate::map::floor::prepare_quest_encounter_ejection(
            runtime,
            cells,
            actor,
            session,
            expected,
            destination,
            &closed,
        ) else {
            return;
        };
        exits.push(proof);
    }
    if exits.is_empty()
        || runtime
            .commit_source_steps_and_project(exits, || true)
            .is_ok()
    {
        states.observer_eye_pair = None;
    }
}

pub(in crate::gameplay_transport) fn advance_observer_eye(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    cells: &NativeEntryMovementCells,
    map: &crate::map::boot::BundleMap,
    now_us: u64,
) {
    let Some(pair) = states.observer_eye_pair else {
        return;
    };
    if now_us >= pair.expires {
        eject_expired_pair(runtime, states, cells, map, pair);
        return;
    }
    if current_pair(runtime, states, now_us).is_none() {
        states.observer_eye_pair = None;
        return;
    }
    let Some(late) = now_us.checked_sub(pair.next_platform_cycle) else {
        return;
    };
    let rounds = 1 + late / PLATFORM_INTERVAL_MICROS;
    let started = pair.expires - ATTEMPT_MICROS;
    let previous_beat = (pair.next_platform_cycle - started) / PLATFORM_INTERVAL_MICROS - 1;
    for &(x, y) in bindings::WEST_FIELD {
        let at = MovementLocalPosition {
            x,
            y,
            floor: bindings::FLOOR,
        };
        if let Some((key, appearance)) = platform_at(map, at)
            && let Some(phase) = PLATFORM_PHASES.iter().position(|&id| id == appearance)
        {
            // Crystal supplies appearances but no local cadence. Vary the period
            // per placement and attempt without drawing from shared RNG state.
            let seed = key.wrapping_add(started).wrapping_mul(0x9e3779b97f4a7c15);
            let period = 1 + (seed >> 32) % 3;
            let transitions =
                (previous_beat + rounds).div_ceil(period) - previous_beat.div_ceil(period);
            let advance = (transitions % PLATFORM_PHASES.len() as u64) as usize;
            if advance != 0 {
                let after = PLATFORM_PHASES[(phase + advance) % PLATFORM_PHASES.len()];
                map.facts.transform_movement_item(key, after);
            }
        }
    }
    states.observer_eye_pair = Some(ObserverEyePair {
        next_platform_cycle: now_us
            .saturating_add(PLATFORM_INTERVAL_MICROS - late % PLATFORM_INTERVAL_MICROS),
        ..pair
    });
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
    states.observer_eye_pair.is_some_and(|pair| {
        pair.partner.0 == actor
            && if now_us >= pair.expires {
                current_player(runtime, states, pair.partner).is_some_and(platform_field)
            } else {
                current_pair(runtime, states, now_us).is_some()
            }
    })
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
        next_platform_cycle: now_us.checked_add(PLATFORM_INTERVAL_MICROS)?,
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
    if states
        .observer_eye_pair
        .is_some_and(|pair| now_us >= pair.expires)
    {
        return;
    }
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
            let pair = pair_on_pads(runtime, states, now_us);
            if pair.is_some() {
                states.owner_wake.notify_one();
            }
            pair
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
    let platform = match cells.index() {
        NativeMovementCollisionIndex::Bundle(index) => platform_at(index.map(), destination),
        _ => None,
    };
    if platform.is_some_and(|(_, appearance)| is_submerged(appearance)) {
        if let Ok(proof) = crate::map::floor::prepare_quest_encounter_ejection(
            runtime,
            cells,
            partner,
            partner_session,
            expected,
            WEST_EXIT,
            blocking,
        ) && runtime.commit_source_step(proof).is_ok()
        {
            states.observer_eye_pair = None;
            states.observer_eye_fall_notice = Some(partner_session);
        }
        return;
    }
    if platform.is_some() {
        // Crystal's platform graphics are blocking borders over water. The
        // ornament's scripted step can land on a visible platform without
        // opening the tile to ordinary movement.
        if runtime.player_step_allowed(partner, partner_session, direction.delta())
            && let Ok(proof) = crate::map::floor::prepare_scripted_quest_teleport_facing(
                runtime,
                cells,
                partner,
                partner_session,
                expected,
                destination,
                false,
                blocking,
                Some(direction.facing()),
                true,
            )
        {
            let _ = runtime.commit_source_step(proof);
        }
        return;
    }
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
