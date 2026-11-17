//! PARITY-TOOLS-1 first slice: carried rope → resolved map spot → movement-owner commit.
use super::*;
use crate::content::{LogicalCell, NativeEntryMovementCells};
use crate::foundation::ChannelRuntimeV1;
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use crate::gameplay_transport::item_move::{LoopMoveRefusal, LoopMoveResult};
use crate::gameplay_transport::loop_move::{self, LoopGroundOverlay};
use crate::gameplay_transport::world_map::{self, MapHandleTarget, MapViewSource};
use crate::map::floor::rope_item_landing;
use crate::map::overlay::TilePos;
use oteryn_simulation_determinism::SemanticTimeMicros;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_rope_tool(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
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
        let lift = {
            let mut runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            resolve_item_lift(
                self, &mut runtime, &states, actor, command, intent, target, &ground, index,
            )
        };
        if let Some(lift) = lift {
            let disposition = match lift {
                Ok((item, from, to)) => match loop_move::relocate_roped_item(
                    self, &permit, actor, command, item, &ground, from, to,
                )
                .await
                {
                    LoopMoveResult::Committed => UseDisposition::Committed,
                    LoopMoveResult::Refused(LoopMoveRefusal::Stale) => UseDisposition::StaleState,
                    LoopMoveResult::Unknown => return Err(()),
                    _ => UseDisposition::NotPossible,
                },
                Err(disposition) => disposition,
            };
            drop(permit);
            if disposition == UseDisposition::Committed {
                self.refresh_terrain_tools().await;
            }
            return Ok(Some(UseOutcome {
                disposition,
                ..UseOutcome::rejected()
            }));
        }
        if !tokio::time::timeout(
            std::time::Duration::from_secs(10),
            self.refresh_pvp_locks(actor, command.game_session_id),
        )
        .await
        .map_err(|_| ())?
        {
            return Ok(Some(UseOutcome::rejected()));
        }
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let disposition = rope_in_channel(
            &mut runtime,
            &mut states,
            self.movement_cells(),
            actor,
            command,
            intent,
            target,
            self.reset_epoch,
            self.owner_now(),
            Some(ground.overlay()),
        );
        if disposition == UseDisposition::Committed {
            self.states
                .chat
                .close_out_of_range(&runtime, command.game_session_id);
        }
        Ok(Some(UseOutcome {
            disposition,
            ..UseOutcome::rejected()
        }))
    }
}

// Crystal's open holes (register_actions.lua:1), with rope spots retaining self-ascent.
const OPEN_HOLES: &[u16] = &[
    294, 369, 370, 385, 394, 411, 412, 413, 432, 433, 435, 482, 483, 594, 595, 609, 610, 615, 868,
    874, 1156, 4824, 7515, 7516, 7517, 7518, 7519, 7520, 7521, 7522, 7737, 7755, 7767, 7768, 8144,
    8690, 8709, 12203, 12961, 17239, 19220, 23364,
];

#[allow(
    clippy::too_many_arguments,
    reason = "current rope USE and its fenced map projection"
)]
fn resolve_item_lift(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    states: &ChannelSpellStates,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &UseIntent,
    target: Option<ItemKey>,
    ground: &LoopGroundOverlay,
    index: &crate::content::item_ref::ItemDefinitionIndex,
) -> Option<Result<([u8; 16], TilePos, TilePos), UseDisposition>> {
    let cells = admission.movement_cells();
    let at = super::super::bundle_map_position(runtime, cells, actor)?;
    let pinned = at.map.facts.pinned();
    let map = MapViewSource {
        overlay: ground.overlay(),
        facts: &pinned,
        content_generation: at.content_generation,
        reset_epoch: admission.reset_epoch,
    };
    let (hole, ordinal) = rope_target(&map, intent, target).ok()?;
    let tile = map.overlay.base().tile(hole.x, hole.y, hole.floor)?;
    let appearance = tile
        .ids()
        .iter()
        .zip(tile.depths())
        .filter(|(_, depth)| **depth == 0)
        .nth(usize::from(ordinal))
        .and_then(|(id, _)| pinned.base_entry(hole, ordinal, *id))?
        .appearance_id;
    if OPEN_HOLES.binary_search(&appearance).is_err() {
        return None;
    }
    if states.is_dead(actor) || !states.spell_idle(runtime, actor, command.game_session_id) {
        return Some(Err(UseDisposition::Rejected));
    }
    if !rope_in_reach(at.at, hole) {
        return Some(Err(UseDisposition::TooFar));
    }
    let facts = ground.facts(&pinned);
    let plan = rope_item_landing(hole, |position| {
        let cell = LogicalCell {
            x: i32::from(position.x),
            y: i32::from(position.y),
            z: -i32::from(position.floor),
        };
        at.map.facts.house_id(position).is_none()
            && !at.map.floors.contains(position)
            && !at.map.facts.doors().blocks(position)
            && cells
                .spell_tiles()
                .lookup(cells.scope(), cell)
                .is_ok_and(|tile| tile.ground_present() && !tile.flags().immovable_block_item)
            && view::compose_tile(ground.overlay(), &facts, position).is_ok_and(|tile| {
                tile.is_some_and(|tile| {
                    tile.entries
                        .iter()
                        .filter(|entry| matches!(entry.source, view::EntrySource::Ground(_)))
                        .count()
                        < crate::durability::ground_cell_limit::GROUND_CELL_ROOT_LIMIT
                })
            })
    });
    let Some((from, to)) = plan else {
        return Some(Err(UseDisposition::NotPossible));
    };
    let native = admission.active_generation?.native_gameplay()?;
    let item = view::compose_tile(ground.overlay(), &facts, from)
        .ok()
        .flatten()
        .and_then(|tile| {
            tile.entries.iter().rev().find_map(|entry| {
                let view::EntrySource::Ground(item) = entry.source else {
                    return None;
                };
                let MapDefinition::Item(reference) = entry.facts.definition else {
                    return None;
                };
                let (key, revision) = index.definition_at(reference)?;
                let policy = native.item_policy(key, revision)?;
                use crate::content::ReferenceItemField as Fact;
                matches!(&policy.record().semantics.physical, Fact::Known(physical)
                    if physical.movable == Fact::Known(true))
                .then_some(item)
            })
        });
    Some(
        item.map(|item| (item, from, to))
            .ok_or(UseDisposition::NotPossible),
    )
}

fn rope_target(
    map: &MapViewSource<'_, impl MapFacts>,
    intent: &UseIntent,
    target: Option<ItemKey>,
) -> Result<(TilePos, u8), UseDisposition> {
    match &intent.use_with_map_item {
        Some(MapUseTarget::Object(object)) => {
            world_map::resolve_map_target(map, object, |_| None::<()>)
                .map(|target| (target.pos, target.ordinal))
                .map_err(|_| UseDisposition::StaleState)
        }
        Some(MapUseTarget::Item(_)) => {
            match target.and_then(|key| world_map::resolve_map_handle(map, key).ok()) {
                Some(MapHandleTarget::Base { pos, ordinal, .. }) => Ok((pos, ordinal)),
                _ => Err(UseDisposition::StaleState),
            }
        }
        None => Err(UseDisposition::NothingToUse),
    }
}

fn rope_in_reach(
    at: crate::gameplay_transport::world_spatial::ActorPosition,
    spot: TilePos,
) -> bool {
    i16::from(spot.floor) == at.floor
        && at.x.abs_diff(i32::from(spot.x)) <= 1
        && at.y.abs_diff(i32::from(spot.y)) <= 1
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
                "oteryn:item.tibia.i3003" | "oteryn:item.tibia.i646"
            ) && index
                .definition_ref(
                    &definition.family,
                    &definition.production_key,
                    &definition.revision_ref,
                )
                .is_some()
        })
}

#[allow(
    clippy::too_many_arguments,
    reason = "the existing owner turn's resolved facts"
)]
fn rope_in_channel(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    cells: &NativeEntryMovementCells,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &UseIntent,
    target: Option<ItemKey>,
    reset_epoch: u64,
    now: SemanticTimeMicros,
    overlay: Option<&crate::map::overlay::ChannelOverlay>,
) -> UseDisposition {
    let session = command.game_session_id;
    if states.is_dead(actor) || !states.spell_idle(runtime, actor, session) {
        return UseDisposition::Rejected;
    }
    let Some(at) = crate::gameplay_transport::bundle_map_position(runtime, cells, actor) else {
        return UseDisposition::NothingToUse;
    };
    let facts = at.map.facts.pinned();
    let map = crate::gameplay_transport::world_map::MapViewSource {
        overlay: overlay.unwrap_or(&at.map.overlay),
        facts: &facts,
        content_generation: at.content_generation,
        reset_epoch,
    };
    let (spot, _) = match rope_target(&map, intent, target) {
        Ok(target) => target,
        Err(disposition) => return disposition,
    };
    if !rope_in_reach(at.at, spot) {
        return UseDisposition::TooFar;
    }
    let spot = LogicalCell {
        x: i32::from(spot.x),
        y: i32::from(spot.y),
        z: -i32::from(spot.floor),
    };
    let Ok(expected) = runtime.read_actor_position(actor) else {
        return UseDisposition::Rejected;
    };
    let blocking = crate::map::floor::BundleDoors(&at.map.facts);
    let blocked = crate::gameplay_transport::pvp_live::pz_blocked(states, actor, session, now);
    let proof = match crate::map::floor::prepare_bundle_rope(
        runtime, cells, actor, session, expected, spot, blocked, &blocking,
    ) {
        Ok(proof) => proof,
        Err(crate::movement::MovementError::NotQualified) => return UseDisposition::NothingToUse,
        Err(_) => return UseDisposition::NotPossible,
    };
    use oteryn_gameplay_spell_rules::cast::{STAIRHOP_COOLDOWN_KEY, STAIRHOP_MICROS};
    let Ok(until) = now.checked_add(STAIRHOP_MICROS) else {
        return UseDisposition::Rejected;
    };
    let cooldowns = states.get(runtime, actor, session).map(|state| {
        let mut next = state.cooldowns.clone();
        next.groups.insert(STAIRHOP_COOLDOWN_KEY.into(), until);
        next
    });
    if runtime.commit_source_step(proof).is_err() {
        return UseDisposition::Rejected;
    }
    if let Some(cooldowns) = cooldowns
        && let Some(state) = states.get_mut(runtime, actor, session)
    {
        state.cooldowns = cooldowns;
    }
    UseDisposition::Committed
}

#[cfg(test)]
#[path = "rope_tests.rs"]
mod tests;
