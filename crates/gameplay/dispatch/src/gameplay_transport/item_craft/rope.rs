//! PARITY-TOOLS-1 first slice: carried rope → resolved map spot → movement-owner commit.
use super::*;
use crate::content::{LogicalCell, NativeEntryMovementCells};
use crate::foundation::ChannelRuntimeV1;
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
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
        let Some(_permit) = self.spell_lane_permit().await else {
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
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let disposition = rope_in_channel(
            &mut runtime,
            &mut states,
            self.movement_cells,
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
    let spot = match &intent.use_with_map_item {
        Some(MapUseTarget::Object(object)) => {
            match crate::gameplay_transport::world_map::resolve_map_target(&map, object, |_| {
                None::<()>
            }) {
                Ok(target) => target.pos,
                Err(_) => return UseDisposition::StaleState,
            }
        }
        Some(MapUseTarget::Item(_)) => match target.and_then(|key| {
            crate::gameplay_transport::world_map::resolve_map_handle(&map, key).ok()
        }) {
            Some(crate::gameplay_transport::world_map::MapHandleTarget::Base { pos, .. }) => pos,
            _ => return UseDisposition::StaleState,
        },
        None => return UseDisposition::NothingToUse,
    };
    if i16::from(spot.floor) != at.at.floor
        || at.at.x.abs_diff(i32::from(spot.x)) > 1
        || at.at.y.abs_diff(i32::from(spot.y)) > 1
    {
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
    let blocking = crate::map::floor::BundleDoors(at.map.facts.doors());
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
