//! Carried rope climbs a rope spot or pulls through a resolved open hole.
use super::super::{AdmissionRefusal, AdmissionRetryBudget};
use super::*;
use crate::content::{LogicalCell, NativeEntryMovementCells};
use crate::durability::{DurabilityError, item_use::ItemUseError};
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use oteryn_simulation_determinism::SemanticTimeMicros;

#[path = "rope_pull.rs"]
mod pull;

#[cfg_attr(test, derive(Debug))]
pub(in crate::gameplay_transport) enum RopeUseError {
    Preflight,
    OutcomeUnknown,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) fn exact_rope_spot_placement(
        &self,
        command: UseCommand,
        intent: &UseIntent,
    ) -> Option<u64> {
        let fence = command.item_fence?;
        if !matches!(
            fence.runtime_scope,
            crate::foundation::RuntimeScopeRefV1::Channel { .. }
        ) {
            return None;
        }
        let target @ MapUseTarget::Object(_) = intent.use_with_map_item.as_ref()? else {
            return None;
        };
        let map = self.bundle_map()?;
        let generation = self.active_generation?;
        let facts = map.facts.pinned();
        let view = super::super::world_map::MapViewSource {
            overlay: &map.overlay,
            facts: &facts,
            content_generation: generation.identity().client_artifact_digest(),
            reset_epoch: self.reset_epoch,
        };
        let (spot, placement) = terrain_target(&view, target, None).ok()?;
        placement.filter(|_| is_rope_spot(&map.overlay, &facts, spot))
    }

    pub(in crate::gameplay_transport) async fn use_rope_tool(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, RopeUseError> {
        let Some(fence) = command.item_fence else {
            return Ok(None);
        };
        let Some(native) = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
        else {
            return Ok(None);
        };
        // A definition-selected tool can be classified before the fenced read.
        // A non-rope must reach its own handler even when the item fence is stale.
        if let UseTarget::ItemByDefinition(wanted) = intent.target
            && !native
                .item_index()
                .and_then(|index| index.definition_at(wanted))
                .is_some_and(|(key, _)| rope_definition(key))
        {
            return Ok(None);
        }
        let Some(map) = self.bundle_map() else {
            return Ok(None);
        };
        let permit = match self.try_spell_lane_permit().await {
            Ok(permit) => permit,
            Err(DurabilityError::RootUnavailable | DurabilityError::RootPassDeadlineExceeded) => {
                return Ok(Some(UseOutcome::rejected()));
            }
            Err(_) => return Err(RopeUseError::Preflight),
        };
        let retry = AdmissionRetryBudget::for_command(self.root);
        let inventory = match tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let result = self
                    .root
                    .read_item_use_inventory(self.character, self.holder, fence)
                    .await;
                if !matches!(
                    &result,
                    Err(ItemUseError::Unavailable(DurabilityError::RootUnavailable))
                ) || !retry
                    .retries_unavailable(AdmissionRefusal::Unavailable)
                    .await
                {
                    break result;
                }
            }
        })
        .await
        {
            Ok(Ok(inventory)) => inventory,
            Err(_)
            | Ok(Err(ItemUseError::Unavailable(
                DurabilityError::RootUnavailable | DurabilityError::RootPassDeadlineExceeded,
            ))) => return Ok(Some(UseOutcome::rejected())),
            Ok(Err(_)) => return Err(RopeUseError::Preflight),
        };
        drop(retry);
        let index = {
            let runtime = self.runtime.lock().await;
            pinned_index(&runtime, self.active_generation).ok_or(RopeUseError::Preflight)?
        };
        let Some((definition, tool)) = carried_rope(&inventory, index, intent, source) else {
            return Ok(None);
        };
        if native
            .item_policy(&definition.production_key, &definition.revision_ref)
            .is_none()
        {
            return Ok(None);
        }
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, Some(tool))
        {
            return Ok(Some(UseOutcome::rejected()));
        }
        let ground = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            crate::gameplay_transport::loop_move::loop_ground(
                self,
                map.overlay.base(),
                Some(index),
            ),
        )
        .await
        .map_err(|_| RopeUseError::Preflight)?
        .ok_or(RopeUseError::Preflight)?;
        if let Some(ItemKey::Instance(item)) = target {
            let facts = map.facts.pinned();
            if !ground
                .overlay()
                .ground_entry(&item)
                .is_some_and(|(pos, _)| is_rope_spot(&map.overlay, &facts, pos))
            {
                return Ok(None);
            }
        }
        if !tokio::time::timeout(
            std::time::Duration::from_secs(10),
            self.refresh_pvp_locks(actor, command.game_session_id),
        )
        .await
        .map_err(|_| RopeUseError::Preflight)?
        {
            return Ok(Some(UseOutcome::rejected()));
        }
        let optional_pvp = self
            .special_movement_policy(&permit, actor, command.game_session_id)
            .await;
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let now = self.owner_now();
        let turn = rope_in_channel(
            &mut runtime,
            &mut states,
            self.movement_cells(),
            actor,
            command,
            intent,
            target,
            self.reset_epoch,
            now,
            Some(ground.overlay()),
            Some(&self.states),
            || agent_tool_jams(command, &definition.production_key, tool),
        );
        let disposition = match turn {
            RopeTurn::Refused(UseDisposition::NothingToUse) => return Ok(None),
            RopeTurn::Refused(disposition) => disposition,
            RopeTurn::Jam => {
                drop((states, runtime));
                let encoded = oteryn_protocol_oteryn::world_object::encode_item_use_intent(intent)
                    .map_err(|_| RopeUseError::Preflight)?;
                return self
                    .commit_agent_jam(&permit, command, tool, encoded)
                    .await
                    .map(Some)
                    .map_err(|_| RopeUseError::OutcomeUnknown);
            }
            RopeTurn::Hole(spot) => {
                match pull::pull(
                    &mut runtime,
                    &mut states,
                    self.movement_cells(),
                    map,
                    &ground,
                    &self.states,
                    self.reset_epoch,
                    spot,
                    now,
                    optional_pvp,
                ) {
                    Err(disposition) => disposition,
                    Ok(pull::Pull::Creature) => UseDisposition::Committed,
                    Ok(pull::Pull::Player {
                        actor,
                        session,
                        origin,
                        revision,
                    }) => {
                        self.states.chat.close_out_of_range(&runtime, session);
                        drop((states, runtime));
                        self.close_departed_door(&permit, actor, session, origin, revision)
                            .await;
                        UseDisposition::Committed
                    }
                    Ok(pull::Pull::Item {
                        item,
                        from,
                        landings,
                    }) => 'relocate: {
                        use crate::gameplay_transport::item_move::{
                            LoopMoveRefusal, LoopMoveResult,
                        };
                        use crate::gameplay_transport::loop_move::{self, SourceItemOrigin};

                        drop((states, runtime));
                        let _ground_change =
                            loop_move::GroundChange(Some(&self.states.loop_ground));
                        for to in landings {
                            match loop_move::relocate_source_item(
                                self,
                                &permit,
                                actor,
                                command,
                                item,
                                SourceItemOrigin::Rope(from),
                                to,
                            )
                            .await
                            {
                                LoopMoveResult::Committed => {
                                    break 'relocate UseDisposition::Committed;
                                }
                                LoopMoveResult::Unknown => {
                                    return Err(RopeUseError::OutcomeUnknown);
                                }
                                LoopMoveResult::Refused(LoopMoveRefusal::NoRoom) => continue,
                                LoopMoveResult::TooFar | LoopMoveResult::Refused(_) => break,
                            }
                        }
                        UseDisposition::NotPossible
                    }
                }
            }
            RopeTurn::Moved { origin, revision } => {
                self.states
                    .chat
                    .close_out_of_range(&runtime, command.game_session_id);
                drop((states, runtime));
                self.close_departed_door(&permit, actor, command.game_session_id, origin, revision)
                    .await;
                UseDisposition::Committed
            }
        };
        Ok(Some(UseOutcome {
            disposition,
            ..UseOutcome::rejected()
        }))
    }
}

fn rope_definition(key: &str) -> bool {
    matches!(key, "oteryn:item.tibia.i3003" | "oteryn:item.tibia.i646") || secret_agent_rope(key)
}

/// Crystal 201189f8 Secret Agent tools try rope before shovel, pick and machete.
pub(super) fn secret_agent_rope(key: &str) -> bool {
    oteryn_gameplay_items::secret_agent::jammed_successor(key).is_some()
}

pub(in crate::gameplay_transport) fn agent_tool_jams(
    command: UseCommand,
    key: &str,
    tool: [u8; 16],
) -> bool {
    secret_agent_rope(key)
        && oteryn_gameplay_items::secret_agent::jams(
            *command.game_session_id.as_bytes(),
            command.command_id,
            tool,
        )
}

fn carried_rope<'a>(
    inventory: &'a crate::durability::item_use::ItemUseInventory,
    index: &crate::content::item_ref::ItemDefinitionIndex,
    intent: &UseIntent,
    source: Option<ItemKey>,
) -> Option<(&'a TypedDefinitionRef, [u8; 16])> {
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
        .map(|entry| (&entry.item.definition, entry.item.item_instance_id))
        .filter(|(definition, _)| {
            rope_definition(&definition.production_key)
                && index
                    .definition_ref(
                        &definition.family,
                        &definition.production_key,
                        &definition.revision_ref,
                    )
                    .is_some()
        })
}

/// Resolve a tool's selected entry to its tile, retaining the placement only for base transforms.
pub(super) fn terrain_target(
    map: &crate::gameplay_transport::world_map::MapViewSource<'_, impl view::MapFacts>,
    intent: &MapUseTarget,
    key: Option<ItemKey>,
) -> Result<(crate::map::overlay::TilePos, Option<u64>), UseDisposition> {
    use crate::gameplay_transport::world_map::{
        MapHandleTarget, resolve_map_handle, resolve_map_target,
    };
    match intent {
        MapUseTarget::Object(object) => resolve_map_target(map, object, |_| None::<()>)
            .map(|target| (target.pos, Some(target.placement_key)))
            .map_err(|_| UseDisposition::StaleState),
        MapUseTarget::Item(_) => {
            match resolve_map_handle(map, key.ok_or(UseDisposition::StaleState)?)? {
                MapHandleTarget::Base {
                    pos, placement_key, ..
                } => Ok((pos, Some(placement_key))),
                MapHandleTarget::Added { pos, .. } | MapHandleTarget::Ground { pos, .. } => {
                    Ok((pos, None))
                }
            }
        }
    }
}

fn is_rope_spot(
    overlay: &crate::map::overlay::ChannelOverlay,
    facts: &impl view::MapFacts,
    pos: crate::map::overlay::TilePos,
) -> bool {
    view::compose_tile(overlay, facts, pos)
        .ok()
        .flatten()
        .as_ref()
        .is_some_and(crate::map::floor::is_bundle_rope_spot)
}

#[derive(Debug, PartialEq, Eq)]
enum RopeTurn {
    Refused(UseDisposition),
    Jam,
    Hole(crate::map::overlay::TilePos),
    Moved {
        origin: MovementLocalPosition,
        revision: u64,
    },
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
    channel_states: Option<&crate::gameplay_transport::channel_states::ChannelSessionStates>,
    jam: impl FnOnce() -> bool,
) -> RopeTurn {
    let session = command.game_session_id;
    if states.is_dead(actor)
        || !states.spell_idle(runtime, actor, session)
        || !runtime
            .player_control_facts(actor, session)
            .is_ok_and(|facts| facts.control_loss.is_none())
    {
        return RopeTurn::Refused(UseDisposition::Rejected);
    }
    let Some(at) = crate::gameplay_transport::bundle_map_position(runtime, cells, actor) else {
        return RopeTurn::Refused(UseDisposition::NothingToUse);
    };
    let facts = at.map.facts.pinned();
    let map = crate::gameplay_transport::world_map::MapViewSource {
        overlay: overlay.unwrap_or(&at.map.overlay),
        facts: &facts,
        content_generation: at.content_generation,
        reset_epoch,
    };
    let (spot, placement) = match &intent.use_with_map_item {
        Some(target_intent) => match terrain_target(&map, target_intent, target) {
            Ok(resolved) => resolved,
            Err(_) if matches!(target, Some(ItemKey::Instance(_))) => {
                return RopeTurn::Refused(UseDisposition::NothingToUse);
            }
            Err(disposition) => return RopeTurn::Refused(disposition),
        },
        None => return RopeTurn::Refused(UseDisposition::NothingToUse),
    };
    if i16::from(spot.floor) != at.at.floor
        || at.at.x.abs_diff(i32::from(spot.x)) > 1
        || at.at.y.abs_diff(i32::from(spot.y)) > 1
    {
        return RopeTurn::Refused(UseDisposition::TooFar);
    }
    if is_rope_spot(&at.map.overlay, &facts, spot) {
        if jam() {
            return RopeTurn::Jam;
        }
    } else if placement.is_some_and(|placement| pull::is_hole(&map, placement)) {
        return if jam() {
            RopeTurn::Jam
        } else {
            RopeTurn::Hole(spot)
        };
    }
    let spot = LogicalCell {
        x: i32::from(spot.x),
        y: i32::from(spot.y),
        z: -i32::from(spot.floor),
    };
    let Ok(expected) = runtime.read_actor_position(actor) else {
        return RopeTurn::Refused(UseDisposition::Rejected);
    };
    let blocking = crate::map::floor::BundleDoors(&at.map.facts);
    let door_access = channel_states.map(|states_by_session| {
        crate::gameplay_transport::paced_movement::PlayerDoorAccess {
            inner: &blocking,
            level: states
                .get(runtime, actor, session)
                .map(|state| state.character_facts().level),
            states: states_by_session,
            session,
        }
    });
    let blocking: &dyn crate::map::floor::CellBlocker = door_access
        .as_ref()
        .map_or(&blocking as &dyn crate::map::floor::CellBlocker, |access| {
            access
        });
    let blocked = crate::gameplay_transport::pvp_live::pz_blocked(states, actor, session, now);
    let proof = match crate::map::floor::prepare_bundle_rope(
        runtime, cells, actor, session, expected, spot, blocked, blocking,
    ) {
        Ok(proof) => proof,
        Err(crate::movement::MovementError::NotQualified) => {
            return RopeTurn::Refused(UseDisposition::NothingToUse);
        }
        Err(_) => return RopeTurn::Refused(UseDisposition::NotPossible),
    };
    use oteryn_gameplay_spell_rules::cast::{STAIRHOP_COOLDOWN_KEY, STAIRHOP_MICROS};
    let Ok(until) = now.checked_add(STAIRHOP_MICROS) else {
        return RopeTurn::Refused(UseDisposition::Rejected);
    };
    let cooldowns = states.get(runtime, actor, session).map(|state| {
        let mut next = state.cooldowns.clone();
        next.groups.insert(STAIRHOP_COOLDOWN_KEY.into(), until);
        next
    });
    let Ok(snapshot) = runtime.commit_source_step(proof) else {
        return RopeTurn::Refused(UseDisposition::Rejected);
    };
    if let Some(cooldowns) = cooldowns
        && let Some(state) = states.get_mut(runtime, actor, session)
    {
        state.cooldowns = cooldowns;
    }
    RopeTurn::Moved {
        origin: expected.position(),
        revision: snapshot.revision(),
    }
}

#[cfg(test)]
#[path = "rope_tests.rs"]
mod tests;
