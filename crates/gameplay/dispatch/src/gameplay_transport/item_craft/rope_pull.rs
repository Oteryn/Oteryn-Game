use super::*;
use crate::content::CollisionClass;
use crate::foundation::GameSessionId;
use crate::gameplay_transport::loop_move::LoopGroundOverlay;
use crate::gameplay_transport::world_map::MapViewSource;
use crate::map::boot::BundleMap;
use crate::map::floor::{self, BundleDoors, CellBlocker};
use crate::map::overlay::TilePos;
use crate::map::view::{EntrySource, MapFacts};
use oteryn_gameplay_entities::SceneActorKind;

// Crystal 201189f8 data-global/scripts/lib/register_actions.lua:1,337.
// The selected entry must be the hole, including a currently opened terrain transform.
pub(super) fn is_hole(map: &MapViewSource<'_, impl MapFacts>, placement: u64) -> bool {
    let Some((pos, ordinal)) = view::placement_of(placement) else {
        return false;
    };
    map.overlay
        .base()
        .tile(pos.x, pos.y, pos.floor)
        .and_then(|tile| {
            tile.ids()
                .iter()
                .zip(tile.depths())
                .filter(|(_, depth)| **depth == 0)
                .nth(usize::from(ordinal))
        })
        .and_then(|(&id, _)| map.facts.base_entry(pos, ordinal, id))
        .is_some_and(|entry| {
            matches!(entry.appearance_id,
            294 | 369 | 370 | 385 | 394 | 411..=413 | 432 | 433 | 435 | 482 | 483
            | 594 | 595 | 609 | 610 | 615 | 868 | 874 | 1156 | 4824 | 7515..=7522
            | 7737 | 7755 | 7762 | 7767 | 7768 | 8144 | 8690 | 8709 | 12203 | 12961
            | 17239 | 19220 | 23364)
        })
}

pub(super) enum Pull {
    Item {
        item: ItemKey,
        from: TilePos,
        to: TilePos,
    },
    Player {
        actor: ExactActorRef,
        session: GameSessionId,
        origin: MovementLocalPosition,
        revision: u64,
    },
    Creature,
}

fn position(tile: TilePos) -> MovementLocalPosition {
    MovementLocalPosition {
        x: i32::from(tile.x),
        y: i32::from(tile.y),
        floor: -i16::from(tile.floor),
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "one Channel turn binds the hole, current custody and landing"
)]
pub(super) fn pull(
    runtime: &mut ChannelRuntimeV1,
    states: &ChannelSpellStates,
    cells: &NativeEntryMovementCells,
    map: &BundleMap,
    ground: &LoopGroundOverlay,
    channel: &crate::gameplay_transport::channel_states::ChannelSessionStates,
    reset_epoch: u64,
    spot: TilePos,
    now: SemanticTimeMicros,
) -> Result<Pull, UseDisposition> {
    let impossible = UseDisposition::NotPossible;
    let from = TilePos {
        floor: spot
            .floor
            .checked_sub(1)
            .filter(|&floor| floor >= -15)
            .ok_or(impossible)?,
        ..spot
    };
    let facts = map.facts.pinned();
    let facts = ground.facts(&facts);
    let stack = view::compose_tile(ground.overlay(), &facts, from)
        .map_err(|_| impossible)?
        .ok_or(impossible)?;
    let doors = BundleDoors(&map.facts);
    // Crystal Position:moveUpstairs tries south, then north/east/west and the diagonals.
    let to = [
        (0, 1),
        (0, -1),
        (1, 0),
        (-1, 0),
        (-1, 1),
        (1, 1),
        (-1, -1),
        (1, -1),
    ]
    .into_iter()
    .find_map(|(dx, dy)| {
        let tile = TilePos {
            x: spot.x.checked_add_signed(dx)?,
            y: spot.y.checked_add_signed(dy)?,
            floor: spot.floor,
        };
        let at = position(tile);
        let cell = LogicalCell {
            x: at.x,
            y: at.y,
            z: i32::from(at.floor),
        };
        let stack = view::compose_tile(ground.overlay(), &facts, tile).ok()??;
        (stack.ground_speed > 0
            && stack
                .entries
                .iter()
                .all(|entry| !entry.facts.blocks_projectile)
            && cells.index().lookup(cells.scope(), cell) == Ok(CollisionClass::Walkable)
            && !doors.blocks(cell)
            && !floor::is_teleport(cells, cell)
            && map.facts.house_id(tile).is_none())
        .then_some(tile)
    })
    .ok_or(impossible)?;
    let destination = position(to);

    // Crystal's top visible thing gives a creature precedence over tile items.
    let actors = runtime.scene_actor_census().map_err(|_| impossible)?;
    if let Some(target) = actors.into_iter().rev().find(|entry| {
        let session = match entry.kind {
            SceneActorKind::Player(session) => Some(session),
            _ => None,
        };
        entry.position.position() == position(from)
            && states.native_actor_invisible(runtime, entry.actor, session, now) != Some(true)
    }) {
        if states.is_dead(target.actor)
            || runtime.actor_spell_reserved(target.actor)
            || runtime
                .scene_destination_reserved(destination)
                .unwrap_or(true)
        {
            return Err(impossible);
        }
        return match target.kind {
            SceneActorKind::Npc => Err(impossible),
            SceneActorKind::Creature => {
                let moved = crate::movement::relocate::relocate_creature(
                    runtime,
                    target.actor,
                    cells,
                    &doors,
                    destination,
                )
                .map_err(|_| impossible)?;
                crate::movement::monster_map::apply_committed_step_in(
                    runtime,
                    cells,
                    target.actor,
                    moved.position(),
                );
                map.facts
                    .record_monster_step(crate::map::facts::MonsterStep {
                        actor: target.actor,
                        from: target.position.position(),
                        to: moved.position(),
                    });
                crate::gameplay_transport::paced_movement::monster_trap::apply_monster_trap_steps(
                    runtime,
                    states,
                    map,
                    now.get(),
                );
                Ok(Pull::Creature)
            }
            SceneActorKind::Player(session) => {
                let access = crate::gameplay_transport::paced_movement::PlayerDoorAccess {
                    inner: &doors,
                    level: states
                        .get(runtime, target.actor, session)
                        .map(|state| state.character_facts().level),
                    states: channel,
                    session,
                };
                let cell = LogicalCell {
                    x: destination.x,
                    y: destination.y,
                    z: i32::from(destination.floor),
                };
                if floor::player_door_entry_blocked(cells, cell, &access)
                    || !states.spell_idle(runtime, target.actor, session)
                {
                    return Err(impossible);
                }
                let proof = floor::prepare_quest_callback_teleport(
                    runtime,
                    cells,
                    target.actor,
                    session,
                    target.position,
                    destination,
                    crate::gameplay_transport::pvp_live::pz_blocked(
                        states,
                        target.actor,
                        session,
                        now,
                    ),
                    &access,
                )
                .map_err(|_| impossible)?;
                let after = runtime.commit_source_step(proof).map_err(|_| impossible)?;
                Ok(Pull::Player {
                    actor: target.actor,
                    session,
                    origin: target.position.position(),
                    revision: after.revision(),
                })
            }
        };
    }

    // House furniture and bound authored placements stay with their respective owners.
    if map.facts.house_id(from).is_some() {
        return Err(impossible);
    }
    let entry = stack.entries.last().ok_or(impossible)?;
    let item = match entry.source {
        EntrySource::Ground(id) if entry.facts.pickupable => ItemKey::Instance(id),
        EntrySource::Base {
            placement_key,
            movable: true,
            ..
        } => ItemKey::MapBase {
            bundle_digest: map.overlay.base().digest(),
            placement_key,
            reset_epoch,
        },
        _ => return Err(impossible),
    };
    Ok(Pull::Item { item, from, to })
}
