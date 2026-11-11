//! Crystal rottenEssences.lua:1-28, composed through the existing Quest cursor.
use super::*;
use crate::durability::quest_state::{QuestTransitionOutcome, rotten_essence::RottenEssenceUse};

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Classify a carried essence before the House transform's quantity-one source read.
    /// The existing essence owner rechecks its exact native item and durable fence.
    pub(in crate::gameplay_transport) async fn try_use_house_rotten_essence(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        source: ItemKey,
    ) -> Option<UseOutcome> {
        {
            let runtime = self.runtime.lock().await;
            super::super::item_ref_admission::pinned_index(&runtime, self.active_generation)?;
            crate::durability::consume_transform::HouseActionContext::from_native_owner(
                &runtime,
                actor,
                command.game_session_id,
                command.item_fence?,
            )
            .ok()?;
        }
        let disposition = match Box::pin(use_essence(
            self,
            actor,
            command,
            &UseSource::Handle(source),
        ))
        .await?
        {
            Ok(disposition) | Err(disposition) => disposition,
        };
        Some(UseOutcome {
            disposition,
            ..UseOutcome::rejected()
        })
    }
}

#[derive(Debug, Clone)]
pub(super) struct PendingEssenceUse {
    actor: ExactActorRef,
    session: GameSessionId,
    fence: CurrentCharacterItemFence,
    native: [u8; 32],
    wire: [u8; 32],
    input: RottenEssenceUse,
}

fn binding(source: &UseSource) -> Option<[u8; 32]> {
    let mut hash = Sha256::new();
    hash.update(b"oteryn:rotten-essence-use:v1\0");
    match source {
        UseSource::Handle(ItemKey::Instance(item)) => {
            hash.update([1]);
            hash.update(item);
        }
        UseSource::Definition(definition) => {
            hash.update([2]);
            hash.update(definition.get().to_be_bytes());
        }
        _ => return None,
    }
    Some(hash.finalize().into())
}

pub(super) fn current(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    command: UseCommand,
) -> bool {
    let Some(fence) = command.item_fence else {
        return false;
    };
    if let Ok(house) = runtime.house_actor_state(actor, command.game_session_id) {
        return house.scope == fence.runtime_scope
            && house.generation == fence.scope_ownership_generation
            && runtime
                .house_player_control_facts(actor, command.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none());
    }
    runtime.owner_fence().is_ok()
        && runtime.binding().runtime_scope() == fence.runtime_scope
        && runtime.binding().scope_generation() == fence.scope_ownership_generation
        && runtime
            .player_control_facts(actor, command.game_session_id)
            .is_ok_and(|facts| facts.control_loss.is_none())
}

pub(super) fn gate<'a>(
    states: &'a mut ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
) -> &'a mut ItemUseGate {
    states.item_uses.0.retain(|(actor, session, _)| {
        runtime.player_control_facts(*actor, *session).is_ok()
            || runtime.house_player_control_facts(*actor, *session).is_ok()
    });
    if let Some(index) = states
        .item_uses
        .0
        .iter()
        .position(|(a, s, _)| *a == actor && *s == session)
    {
        return &mut states.item_uses.0[index].2;
    }
    let index = states.item_uses.0.len();
    states
        .item_uses
        .0
        .push((actor, session, ItemUseGate::default()));
    &mut states.item_uses.0[index].2
}

/// Resolve retained PREPARE before selecting another stack. A cancellation or
/// unknown COMMIT retains its exact IDs and keeps that unit unspendable.
async fn settle(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    pending: &PendingEssenceUse,
) -> Result<UseDisposition, UseDisposition> {
    let Some(active) = owner.active_generation else {
        return Err(UseDisposition::Rejected);
    };
    let Some(native) = active.native_gameplay() else {
        return Err(UseDisposition::Rejected);
    };
    let command = UseCommand {
        game_session_id: pending.session,
        command_id: pending.input.item.command_id,
        item_fence: Some(pending.fence),
    };
    {
        let runtime = owner.runtime.lock().await;
        if !current(&runtime, pending.actor, command)
            || native.source_digest() != pending.native
            || !native
                .qualifies_rotten_blood_callbacks(runtime.content_pin().server_artifact_digest())
        {
            return Err(UseDisposition::Rejected);
        }
    }
    let Some(catalogue) = owner.quest_catalogue.as_ref() else {
        return Err(UseDisposition::Rejected);
    };
    let Some(fence) = owner
        .current_quest_fence(pending.session)
        .await
        .ok()
        .flatten()
    else {
        return Err(UseDisposition::Rejected);
    };
    if fence.item_fence() != pending.fence {
        return Err(UseDisposition::Rejected);
    }
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let outcome = slot
        .commit_rotten_blood_essence_use(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            pending.input.clone(),
            std::sync::Arc::clone(catalogue),
        )
        .await;
    drop(slot);
    let disposition = match outcome {
        Ok(QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)) => {
            UseDisposition::Committed
        }
        Ok(QuestTransitionOutcome::Refused(
            oteryn_gameplay_quests::quest::QuestRefusal::StageMismatch,
        )) if pending.input.already_held() => {
            owner
                .states
                .chat
                .push_quest_feedback(pending.session, "sorry not possible.");
            UseDisposition::Committed
        }
        Ok(QuestTransitionOutcome::Refused(_) | QuestTransitionOutcome::ObligationClosed) => {
            UseDisposition::NotPossible
        }
        Err(crate::durability::character_progression::CharacterProgressionError::Unavailable(
            _,
        )) => return Err(UseDisposition::Rejected),
        Err(_) => UseDisposition::Rejected,
    };
    {
        // Presentation and the local completion memo share one await-free owner
        // section. Retained PREPARE also emits once after an unknown COMMIT resolves.
        let mut runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        if disposition == UseDisposition::Committed
            && !pending.input.already_held()
            && current(&runtime, pending.actor, command)
            && let Ok(house) = runtime.house_actor_state(pending.actor, pending.session)
            && house.lease.character_id().as_bytes() == pending.fence.character_id.as_bytes()
            && house.lease.generation() == pending.fence.character_lease_generation
            && let Some(map) = owner.bundle_map()
            && let (Ok(x), Ok(y), Ok(floor)) = (
                house.position.x.try_into(),
                house.position.y.try_into(),
                i8::try_from(house.position.floor)
                    .and_then(|floor| i8::try_from(-i16::from(floor))),
            )
        {
            map.facts.emit_scoped_movement_effect(
                house.scope,
                house.generation,
                crate::map::overlay::TilePos { x, y, floor },
                13,
            );
        }
        if disposition == UseDisposition::Committed
            && !pending.input.already_held()
            && runtime
                .house_actor_state(pending.actor, pending.session)
                .is_err()
            && current(&runtime, pending.actor, command)
            && let Some(at) = super::super::bundle_map_position(
                &mut runtime,
                owner.effective_movement_cells(),
                pending.actor,
            )
            && let (Ok(x), Ok(y), Ok(floor)) = (
                at.at.x.try_into(),
                at.at.y.try_into(),
                at.at.floor.try_into(),
            )
        {
            // Crystal utils_definitions.hpp:68 / rottenEssences.lua:23.
            at.map
                .facts
                .emit_movement_effect(crate::map::overlay::TilePos { x, y, floor }, 13);
        }
        let held = gate(&mut states, &runtime, pending.actor, pending.session);
        held.essence_pending = None;
        held.essence_last = Some((
            pending.input.item.command_id,
            pending.wire,
            pending.native,
            disposition,
        ));
        held.unspendable
            .retain(|item| *item != pending.input.item.source_item_instance_id);
        held.settle(pending.input.item.command_id, disposition, &[]);
    }
    if disposition == UseDisposition::Committed {
        Box::pin(owner.refresh_quest_session(pending.session, false)).await;
    }
    Ok(disposition)
}

pub(super) async fn use_essence(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    source: &UseSource,
) -> Option<Result<UseDisposition, UseDisposition>> {
    let fence = command.item_fence?;
    let pending = {
        let states = owner.states.spell_states.lock().await;
        states
            .item_uses
            .0
            .iter()
            .find(|(a, s, _)| *a == actor && *s == command.game_session_id)
            .and_then(|(_, _, gate)| gate.essence_pending.as_deref())
            .cloned()
    };
    let Some(_permit) = owner.spell_lane_permit().await else {
        return Some(Err(UseDisposition::Rejected));
    };
    {
        let runtime = owner.runtime.lock().await;
        if !current(&runtime, actor, command) {
            return Some(Err(UseDisposition::Rejected));
        }
        let states = owner.states.spell_states.lock().await;
        if pending.is_none()
            && let Some((_, _, gate)) = states
                .item_uses
                .0
                .iter()
                .find(|(a, s, _)| *a == actor && *s == command.game_session_id)
            && let Some((id, wire, native, disposition)) = gate.essence_last
            && id == command.command_id
        {
            return Some(
                if binding(source) == Some(wire)
                    && owner
                        .active_generation
                        .and_then(|active| active.native_gameplay())
                        .is_some_and(|current| current.source_digest() == native)
                {
                    Ok(disposition)
                } else {
                    Err(UseDisposition::Rejected)
                },
            );
        }
    }

    if let Some(pending) = pending {
        match Box::pin(settle(owner, &pending)).await {
            Ok(result) if pending.input.item.command_id == command.command_id => {
                return Some(if binding(source) == Some(pending.wire) {
                    Ok(result)
                } else {
                    Err(UseDisposition::Rejected)
                });
            }
            Ok(_) => {}
            Err(error) => return Some(Err(error)),
        }
    }
    let active = owner.active_generation?;
    let native = active.native_gameplay()?;
    let is_house = {
        let runtime = owner.runtime.lock().await;
        if !current(&runtime, actor, command) {
            return Some(Err(UseDisposition::Rejected));
        }
        if let UseSource::Definition(definition) = source {
            let index = super::super::item_ref_admission::pinned_index(&runtime, Some(active))?;
            let (key, _) = index.definition_at(*definition)?;
            oteryn_gameplay_quests::quest::rotten_blood::essence(key)?;
        }
        runtime
            .house_actor_state(actor, command.game_session_id)
            .is_ok()
    };
    let inventory = if is_house {
        owner
            .root
            .read_hireling_use_inventory(
                owner.character,
                owner.holder,
                fence,
                *owner.channel_id.as_bytes(),
            )
            .await
            .ok()
    } else {
        owner
            .root
            .read_item_use_inventory(owner.character, owner.holder, fence)
            .await
            .ok()
    }?;
    let source_id = match source {
        UseSource::Handle(ItemKey::Instance(id)) => *id,
        UseSource::Definition(definition) => {
            let runtime = owner.runtime.lock().await;
            let index = super::super::item_ref_admission::pinned_index(&runtime, Some(active))?;
            let (key, revision) = index.definition_at(*definition)?;
            let definition = TypedDefinitionRef {
                family: "Item".into(),
                production_key: key.into(),
                revision_ref: revision.into(),
            };
            let mut states = owner.states.spell_states.lock().await;
            match inventory.first_unreserved(
                &definition,
                &gate(&mut states, &runtime, actor, command.game_session_id).unspendable,
            ) {
                Ok(item) => item,
                Err(_) => return Some(Err(UseDisposition::NothingToUse)),
            }
        }
        _ => return None,
    };
    let entry = inventory
        .backpack
        .as_ref()
        .into_iter()
        .flat_map(|b| b.entries.iter())
        .chain(inventory.nested.iter().map(|(_, e)| e))
        .find(|e| e.item.item_instance_id == source_id)?;
    oteryn_gameplay_quests::quest::rotten_blood::essence(&entry.item.definition.production_key)?;
    if !native.qualifies_rotten_blood_callbacks(active.identity().server_artifact_digest())
        || native
            .item_policy(
                &entry.item.definition.production_key,
                &entry.item.definition.revision_ref,
            )
            .is_none()
        || !owner
            .states
            .player_trades
            .physical(command.game_session_id, Some(source_id))
    {
        return Some(Err(UseDisposition::Rejected));
    }
    let (Ok(transaction_id), Ok(event_id), Ok(occurred_at_unix_ms)) = (
        crate::operator_files::uuid_v7(),
        crate::operator_files::uuid_v7(),
        unix_ms(),
    ) else {
        return Some(Err(UseDisposition::Rejected));
    };
    let pending = PendingEssenceUse {
        actor,
        session: command.game_session_id,
        fence,
        native: native.source_digest(),
        wire: binding(source)?,
        input: RottenEssenceUse::new(ItemUseRequest {
            kind: ItemUseKind::RottenEssence,
            command_id: command.command_id,
            content_revision: inventory.content_revision.clone(),
            ruleset_revision: inventory.ruleset_revision.clone(),
            sim_revision: entry_chest::SIM_REVISION.into(),
            transaction_id,
            event_id,
            occurred_at_unix_ms,
            server_build_id: SERVER_BUILD_ID.into(),
            source_item_instance_id: source_id,
            source_definition: entry.item.definition.clone(),
            source_quantity_before: entry.item.quantity,
            flask: ItemUseFlaskPlan::None,
        }),
    };
    {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        if !current(&runtime, actor, command)
            || states.is_dead(actor)
            || states.has_pending_spell_commit(actor, command.game_session_id)
        {
            return Some(Err(UseDisposition::Rejected));
        }
        let gate = gate(&mut states, &runtime, actor, command.game_session_id);
        if let Err(error) = gate.admit(command.command_id, owner.owner_ms()) {
            return Some(Err(error));
        }
        if gate.unspendable.contains(&source_id) {
            return Some(Err(UseDisposition::StaleState));
        }
        if let Err(error) = gate.start_drink(owner.owner_ms()) {
            return Some(Err(error));
        }
        gate.unspendable.push(source_id);
        gate.essence_pending = Some(Box::new(pending.clone()));
    }
    Some(Box::pin(settle(owner, &pending)).await)
}
