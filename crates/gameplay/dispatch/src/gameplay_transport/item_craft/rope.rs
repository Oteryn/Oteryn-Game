//! Carried rope USE on a rope spot or a qualifying opening.
use super::super::loop_move::{self, LoopGroundOverlay, SourceItemOrigin};
use super::*;
use crate::content::native_gameplay::NativeGameplayState;
use crate::content::{LogicalCell, NativeEntryMovementCells, ReferenceItemField};
use crate::durability::item_loop_move::LoopMoveReplay;
use crate::foundation::ChannelRuntimeV1;
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use crate::gameplay_transport::item_move::LoopMoveResult;
use crate::map::floor::{self, BundleDoors};
use crate::map::overlay::{ChannelOverlay, TilePos};
use crate::map::view::EntrySource;
use crate::movement::{MovementError, relocate};
use oteryn_simulation_determinism::SemanticTimeMicros;

// Crystal 201189f8: data-global/scripts/lib/register_actions.lua, holeId and onUseRope.
const ROPE_OPENINGS: &[u16] = &[
    294, 369, 370, 385, 394, 411, 412, 413, 432, 433, 435, 482, 483, 594, 595, 609, 610, 615, 868,
    874, 1156, 4824, 7515, 7516, 7517, 7518, 7519, 7520, 7521, 7522, 7737, 7755, 7762, 7767, 7768,
    8144, 8690, 8709, 12203, 12961, 17239, 19220, 23364,
];

enum RopeTurn {
    Done(UseDisposition),
    MovedPlayer(crate::foundation::GameSessionId),
    PullItem {
        item: ItemKey,
        from: TilePos,
        to: TilePos,
    },
}

struct RopeUse<'a> {
    command: UseCommand,
    intent: &'a UseIntent,
    target: Option<ItemKey>,
    reset_epoch: u64,
    now: SemanticTimeMicros,
    overlay: Option<&'a ChannelOverlay>,
    ground: Option<RopeGround<'a>>,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_rope_tool(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        if intent.use_with_map_item.is_none() {
            return Ok(None);
        }
        let Some(fence) = command.item_fence else {
            return Ok(None);
        };
        let Some(native) = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
        else {
            return Ok(None);
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return Err(());
        };
        let inventory = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            self.root
                .read_item_use_inventory(self.character, self.holder, fence),
        )
        .await
        .map_err(|_| ())?
        .map_err(|_| ())?;
        let index = {
            let runtime = self.runtime.lock().await;
            pinned_index(&runtime, self.active_generation).ok_or(())?
        };
        let Some(definition) = carried_rope(&inventory, index, intent, source) else {
            return Ok(None);
        };
        if native
            .item_policy(&definition.production_key, &definition.revision_ref)
            .is_none()
        {
            return Ok(None);
        }
        let Some(map) = self.bundle_map() else {
            return Ok(Some(UseOutcome {
                disposition: UseDisposition::NothingToUse,
                ..UseOutcome::rejected()
            }));
        };
        let ground = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            crate::gameplay_transport::loop_move::loop_ground(
                self,
                map.overlay.base(),
                Some(index),
            ),
        )
        .await
        .map_err(|_| ())?
        .ok_or(())?;
        if !tokio::time::timeout(
            std::time::Duration::from_secs(10),
            self.refresh_pvp_locks(actor, command.game_session_id),
        )
        .await
        .map_err(|_| ())?
        {
            return Ok(Some(UseOutcome::rejected()));
        }
        let item_command = rope_item_command(command, intent)?;
        let replay = self
            .root
            .read_item_loop_move_receipt(
                self.character,
                fence.character_id,
                CommandRef::new(
                    command.game_session_id,
                    CommandId::new(item_command.command_id).map_err(|_| ())?,
                ),
            )
            .await
            .map_err(|_| ())?;
        let disposition = match replay {
            LoopMoveReplay::Moved => UseDisposition::Committed,
            LoopMoveReplay::Conflicting => UseDisposition::Rejected,
            LoopMoveReplay::Absent => {
                let turn = {
                    let mut runtime = self.runtime.lock().await;
                    let mut states = self.states.spell_states.lock().await;
                    let turn = rope_in_channel(
                        &mut runtime,
                        &mut states,
                        self.movement_cells(),
                        actor,
                        RopeUse {
                            command,
                            intent,
                            target,
                            reset_epoch: self.reset_epoch,
                            now: self.owner_now(),
                            overlay: Some(ground.overlay()),
                            ground: Some(RopeGround {
                                overlay: &ground,
                                native,
                                reset_epoch: self.reset_epoch,
                            }),
                        },
                    );
                    if let RopeTurn::MovedPlayer(session) = turn {
                        self.states.chat.close_out_of_range(&runtime, session);
                    }
                    turn
                };
                match turn {
                    RopeTurn::Done(UseDisposition::NothingToUse)
                        if definition.production_key != "oteryn:item.tibia.i3003"
                            && definition.production_key != "oteryn:item.tibia.i646" =>
                    {
                        return Ok(None);
                    }
                    RopeTurn::Done(disposition) => disposition,
                    RopeTurn::MovedPlayer(_) => UseDisposition::Committed,
                    RopeTurn::PullItem { item, from, to } => {
                        match loop_move::relocate_scripted_item(
                            self,
                            &permit,
                            actor,
                            item_command,
                            item,
                            SourceItemOrigin::RopeBelow(from),
                            to,
                        )
                        .await
                        {
                            LoopMoveResult::Committed => UseDisposition::Committed,
                            LoopMoveResult::Unknown => return Err(()),
                            _ => UseDisposition::NotPossible,
                        }
                    }
                }
            }
        };
        Ok(Some(UseOutcome {
            disposition,
            ..UseOutcome::rejected()
        }))
    }
}

// A retry may find a different item at the top of the source stack. Check the
// item-owner receipt bound to the original USE before resolving a fresh pull.
fn rope_item_command(command: UseCommand, intent: &UseIntent) -> Result<UseCommand, ()> {
    let bytes =
        oteryn_protocol_oteryn::world_object::encode_item_use_intent(intent).map_err(|_| ())?;
    let digest = Sha256::new()
        .chain_update(b"oteryn:rope-pull-item:v1")
        .chain_update(command.command_id.to_be_bytes())
        .chain_update(bytes)
        .finalize();
    let id = u64::from_be_bytes(digest[..8].try_into().map_err(|_| ())?).max(1);
    Ok(UseCommand {
        command_id: id,
        ..command
    })
}

fn carried_rope<'a>(
    inventory: &'a crate::durability::item_use::ItemUseInventory,
    index: &crate::content::item_ref::ItemDefinitionIndex,
    intent: &UseIntent,
    source: Option<ItemKey>,
) -> Option<&'a TypedDefinitionRef> {
    let tool = match intent.target {
        UseTarget::Item(_) => match source {
            Some(ItemKey::Instance(id)) => Some(id),
            _ => None,
        },
        UseTarget::ItemByDefinition(wanted) => {
            index.definition_at(wanted).and_then(|(key, revision)| {
                inventory
                    .first_unreserved(
                        &TypedDefinitionRef {
                            family: "Item".into(),
                            production_key: key.into(),
                            revision_ref: revision.into(),
                        },
                        &[],
                    )
                    .ok()
            })
        }
        _ => None,
    }?;
    inventory
        .backpack
        .iter()
        .flat_map(|bag| &bag.entries)
        .chain(inventory.nested.iter().map(|(_, entry)| entry))
        .find(|entry| entry.item.item_instance_id == tool && entry.item.quantity > 0)
        .map(|entry| &entry.item.definition)
        .filter(|definition| {
            matches!(
                definition.production_key.as_str(),
                "oteryn:item.tibia.i646"
                    | "oteryn:item.tibia.i3003"
                    | "oteryn:item.tibia.i9594"
                    | "oteryn:item.tibia.i9596"
                    | "oteryn:item.tibia.i9598"
            ) && index
                .definition_ref(
                    &definition.family,
                    &definition.production_key,
                    &definition.revision_ref,
                )
                .is_some()
        })
}

fn rope_in_channel(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    cells: &NativeEntryMovementCells,
    actor: ExactActorRef,
    use_: RopeUse<'_>,
) -> RopeTurn {
    let RopeUse {
        command,
        intent,
        target,
        reset_epoch,
        now,
        overlay,
        ground,
    } = use_;
    let done = RopeTurn::Done;
    let session = command.game_session_id;
    if !runtime
        .player_control_facts(actor, session)
        .is_ok_and(|facts| facts.control_loss.is_none())
        || states.is_dead(actor)
        || !states.spell_idle(runtime, actor, session)
    {
        return done(UseDisposition::Rejected);
    }
    let Some(at) = crate::gameplay_transport::bundle_map_position(runtime, cells, actor) else {
        return done(UseDisposition::NothingToUse);
    };
    let facts = at.map.facts.pinned();
    let map = crate::gameplay_transport::world_map::MapViewSource {
        overlay: overlay.unwrap_or(&at.map.overlay),
        facts: &facts,
        content_generation: at.content_generation,
        reset_epoch,
    };
    let (spot, ordinal) = match &intent.use_with_map_item {
        Some(MapUseTarget::Object(object)) => {
            match crate::gameplay_transport::world_map::resolve_map_target(&map, object, |_| {
                None::<()>
            }) {
                Ok(target) => (target.pos, target.ordinal),
                Err(_) => return done(UseDisposition::StaleState),
            }
        }
        Some(MapUseTarget::Item(_)) => match target.and_then(|key| {
            crate::gameplay_transport::world_map::resolve_map_handle(&map, key).ok()
        }) {
            Some(crate::gameplay_transport::world_map::MapHandleTarget::Base {
                pos,
                ordinal,
                ..
            }) => (pos, ordinal),
            _ => return done(UseDisposition::StaleState),
        },
        None => return done(UseDisposition::NothingToUse),
    };
    if i16::from(spot.floor) != at.at.floor
        || at.at.x.abs_diff(i32::from(spot.x)) > 1
        || at.at.y.abs_diff(i32::from(spot.y)) > 1
    {
        return done(UseDisposition::TooFar);
    }
    let cell = LogicalCell {
        x: i32::from(spot.x),
        y: i32::from(spot.y),
        z: -i32::from(spot.floor),
    };
    let Ok(expected) = runtime.read_actor_position(actor) else {
        return done(UseDisposition::Rejected);
    };
    let blocking = crate::map::floor::BundleDoors(&at.map.facts);
    let blocked = crate::gameplay_transport::pvp_live::pz_blocked(states, actor, session, now);
    let proof = match crate::map::floor::prepare_bundle_rope(
        runtime, cells, actor, session, expected, cell, blocked, &blocking,
    ) {
        Ok(proof) => proof,
        Err(MovementError::NotQualified) => {
            let Some(appearance) = map
                .overlay
                .base()
                .tile(spot.x, spot.y, spot.floor)
                .and_then(|tile| {
                    tile.ids()
                        .iter()
                        .zip(tile.depths())
                        .filter(|(_, depth)| **depth == 0)
                        .nth(usize::from(ordinal))
                        .and_then(|(id, _)| facts.base_entry(spot, ordinal, *id))
                })
                .map(|entry| entry.appearance_id)
            else {
                return done(UseDisposition::StaleState);
            };
            if !ROPE_OPENINGS.contains(&appearance) {
                return done(UseDisposition::NothingToUse);
            }
            let Some(below_floor) = spot.floor.checked_sub(1) else {
                return done(UseDisposition::NotPossible);
            };
            let below = TilePos {
                floor: below_floor,
                ..spot
            };
            if map
                .overlay
                .base()
                .tile(below.x, below.y, below.floor)
                .is_none()
            {
                return done(UseDisposition::NotPossible);
            }
            return pull_from_below(runtime, states, cells, at.map, below, now, ground)
                .unwrap_or_else(done);
        }
        Err(_) => return done(UseDisposition::NotPossible),
    };
    use oteryn_gameplay_spell_rules::cast::{STAIRHOP_COOLDOWN_KEY, STAIRHOP_MICROS};
    let Ok(until) = now.checked_add(STAIRHOP_MICROS) else {
        return done(UseDisposition::Rejected);
    };
    let cooldowns = states.get(runtime, actor, session).map(|state| {
        let mut next = state.cooldowns.clone();
        next.groups.insert(STAIRHOP_COOLDOWN_KEY.into(), until);
        next
    });
    if runtime.commit_source_step(proof).is_err() {
        return done(UseDisposition::Rejected);
    }
    if let Some(cooldowns) = cooldowns
        && let Some(state) = states.get_mut(runtime, actor, session)
    {
        state.cooldowns = cooldowns;
    }
    RopeTurn::MovedPlayer(session)
}

struct RopeGround<'a> {
    overlay: &'a LoopGroundOverlay,
    native: &'a NativeGameplayState,
    reset_epoch: u64,
}

fn pull_from_below(
    runtime: &mut ChannelRuntimeV1,
    states: &ChannelSpellStates,
    cells: &NativeEntryMovementCells,
    map: &crate::map::boot::BundleMap,
    below: TilePos,
    now: SemanticTimeMicros,
    ground: Option<RopeGround<'_>>,
) -> Result<RopeTurn, UseDisposition> {
    let impossible = UseDisposition::NotPossible;
    let source = LogicalCell {
        x: i32::from(below.x),
        y: i32::from(below.y),
        z: -i32::from(below.floor),
    };
    let blocker = BundleDoors(&map.facts);
    let positioned = runtime
        .positioned_melee_census()
        .map_err(|_| UseDisposition::Rejected)?;
    if let Some((subject, from, session)) = positioned.into_iter().rev().find(|(actor, from, _)| {
        let at = from.position();
        from.context() == runtime.pinned_movement_context()
            && (at.x, at.y, i32::from(at.floor)) == (source.x, source.y, source.z)
            && runtime
                .actor_staff_visibility(*actor)
                .is_ok_and(|(ghost, invisible, _)| !ghost && !invisible)
    }) {
        let unreserved = match session {
            Some(session) => states.spell_idle(runtime, subject, session),
            None => runtime.assert_actor_spell_unreserved(subject).is_ok(),
        };
        if states.is_dead(subject) || !unreserved {
            return Err(impossible);
        }
        let pz_blocked = session.is_some_and(|session| {
            crate::gameplay_transport::pvp_live::pz_blocked(states, subject, session, now)
        });
        let to = floor::bundle_upstairs_landing(cells, source, pz_blocked, &blocker, |to| {
            if session.is_none() && !relocate::creature_cell_admissible(cells, &blocker, to) {
                return Ok(false);
            }
            runtime
                .position_occupied_by_other(subject, to)
                .map(|occupied| !occupied)
                .map_err(MovementError::Actor)
        })
        .map_err(|_| impossible)?;
        if let Some(session) = session {
            let proof = floor::prepare_quest_callback_teleport(
                runtime, cells, subject, session, from, to, pz_blocked, &blocker,
            )
            .map_err(|_| impossible)?;
            runtime
                .commit_source_step(proof)
                .map_err(|_| UseDisposition::Rejected)?;
            return Ok(RopeTurn::MovedPlayer(session));
        }
        relocate::relocate_creature(runtime, subject, cells, &blocker, to)
            .map_err(|_| impossible)?;
        return Ok(RopeTurn::Done(UseDisposition::Committed));
    }
    if runtime
        .npc_atlas_positions()
        .any(|(_, at)| (at.x, at.y, i32::from(at.floor)) == (source.x, source.y, source.z))
    {
        return Err(impossible);
    }
    let item = top_down_item(map, below, ground.ok_or(impossible)?).ok_or(impossible)?;
    let to = floor::bundle_upstairs_landing(cells, source, false, &blocker, |_| Ok(true))
        .map_err(|_| impossible)?;
    let to = TilePos {
        x: u16::try_from(to.x).map_err(|_| impossible)?,
        y: u16::try_from(to.y).map_err(|_| impossible)?,
        floor: i8::try_from(-to.floor).map_err(|_| impossible)?,
    };
    Ok(RopeTurn::PullItem {
        item,
        from: below,
        to,
    })
}

fn top_down_item(
    map: &crate::map::boot::BundleMap,
    below: TilePos,
    ground: RopeGround<'_>,
) -> Option<ItemKey> {
    let pinned = map.facts.pinned();
    let facts = ground.overlay.facts(&pinned);
    let tile = view::compose_tile(ground.overlay.overlay(), &facts, below).ok()??;
    let index = ground.native.item_index()?;
    let (entry, record) = tile.entries.iter().rev().find_map(|entry| {
        let MapDefinition::Item(reference) = entry.facts.definition else {
            return None;
        };
        let (key, revision) = index.definition_at(reference)?;
        let record = ground.native.item_policy(key, revision)?.record();
        (record.attributes.always_on_top != Some(true)).then_some((entry, record))
    })?;
    if !matches!(&record.semantics.physical, ReferenceItemField::Known(physical)
        if physical.movable == ReferenceItemField::Known(true))
    {
        return None;
    }
    match entry.source {
        EntrySource::Ground(id) => Some(ItemKey::Instance(id)),
        EntrySource::Base {
            placement_key,
            movable: true,
            ..
        } => Some(ItemKey::MapBase {
            bundle_digest: map.overlay.base().digest(),
            placement_key,
            reset_epoch: ground.reset_epoch,
        }),
        _ => None,
    }
}

#[cfg(test)]
#[path = "rope_tests.rs"]
mod tests;
