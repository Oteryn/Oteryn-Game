//! Carried actions use the existing quest receipt and item replacement owners.
use super::super::connection::FreshAdmissionAuthority;
use super::*;
use crate::durability::item_transfer::ItemWeightLookup;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::quest_world_callback::{
    CarriedTargetContext, CarriedUseContext, QuestWorldContext,
};
use crate::foundation::{CommandId, CommandRef};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Position, Trigger};
use oteryn_protocol_oteryn::world_object::WorldObjectTarget;

#[derive(Clone)]
pub(in crate::gameplay_transport) struct Pending {
    actor: ExactActorRef,
    command: CommandRef,
    source: [u8; 16],
    intent: Vec<u8>,
    callback: String,
    context: QuestWorldContext,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
}

fn selection(source: &UseSource, target: Option<&WorldObjectTarget>) -> Option<Vec<u8>> {
    let selection = match source {
        UseSource::Definition(definition) => {
            let mut bytes = vec![0];
            bytes.extend_from_slice(&definition.get().to_be_bytes());
            Some(bytes)
        }
        UseSource::Handle(ItemKey::Instance(instance)) => {
            let mut bytes = vec![1];
            bytes.extend_from_slice(instance);
            Some(bytes)
        }
        _ => None,
    }?;
    if let Some(target) = target {
        let mut digest = Sha256::new();
        digest.update(b"oteryn:carried-world-target-intent:v1\0");
        digest.update(&selection);
        digest.update(u16::try_from(target.placement.len()).ok()?.to_be_bytes());
        digest.update(&target.placement);
        digest.update(target.expected_revision.to_be_bytes());
        Some(digest.finalize().to_vec())
    } else {
        Some(selection)
    }
}

async fn clear(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    disposition: UseDisposition,
) {
    if disposition == UseDisposition::Committed {
        admission.states.loop_ground.invalidate();
    }
    let previous = admission
        .states
        .quest_callbacks
        .carried
        .lock()
        .ok()
        .and_then(|mut pending| pending.remove(&command.game_session_id));
    let runtime = admission.runtime.lock().await;
    let mut states = admission.states.spell_states.lock().await;
    if let Some(gate) = states
        .item_uses
        .gate(&runtime, actor, command.game_session_id)
    {
        if let Some(previous) = previous {
            gate.unspendable.retain(|source| *source != previous.source);
        }
        gate.in_flight_since_ms = None;
        gate.last = Some((command.command_id, disposition));
    }
}

pub(super) async fn use_source(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    source: &UseSource,
) -> Option<Result<UseDisposition, UseDisposition>> {
    admission.quest_catalogue.as_ref()?;
    run(admission, actor, command, source, None).await
}

pub(super) async fn use_on_world(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    source: ItemKey,
    target: &WorldObjectTarget,
) -> Option<UseOutcome> {
    admission.quest_catalogue.as_ref()?;
    let result = run(
        admission,
        actor,
        command,
        &UseSource::Handle(source),
        Some(target),
    )
    .await?;
    Some(outcome(result))
}

pub(in crate::gameplay_transport) async fn use_definition_on_world(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    definition: std::num::NonZeroU32,
    target: &WorldObjectTarget,
) -> Option<UseOutcome> {
    admission.quest_catalogue.as_ref()?;
    let result = run(
        admission,
        actor,
        command,
        &UseSource::Definition(definition),
        Some(target),
    )
    .await?;
    Some(outcome(result))
}

fn outcome(result: Result<UseDisposition, UseDisposition>) -> UseOutcome {
    let (Ok(disposition) | Err(disposition)) = result;
    UseOutcome {
        disposition,
        ..UseOutcome::rejected()
    }
}

async fn run(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    source: &UseSource,
    target: Option<&WorldObjectTarget>,
) -> Option<Result<UseDisposition, UseDisposition>> {
    let intent = selection(source, target)?;
    let result = run_inner(admission, actor, command, source, intent, target).await;
    match result {
        Ok(value) => value.map(Ok),
        Err(error) => Some(Err(error)),
    }
}

async fn run_inner(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    source: &UseSource,
    intent: Vec<u8>,
    target: Option<&WorldObjectTarget>,
) -> Result<Option<UseDisposition>, UseDisposition> {
    let trigger = if target.is_some() {
        Trigger::UseCarriedOnTarget
    } else {
        Trigger::UseCarried
    };
    let command_ref = CommandRef::new(
        command.game_session_id,
        CommandId::new(command.command_id).map_err(|_| UseDisposition::Rejected)?,
    );
    let pending = admission
        .states
        .quest_callbacks
        .carried
        .lock()
        .map_err(|_| UseDisposition::Rejected)?
        .get(&command.game_session_id)
        .cloned();
    if pending
        .as_ref()
        .is_some_and(|pending| pending.command != command_ref || pending.intent != intent)
    {
        return Err(UseDisposition::Exhausted);
    }
    {
        let runtime = admission.runtime.lock().await;
        if !runtime
            .player_control_facts(actor, command.game_session_id)
            .is_ok_and(|facts| facts.control_loss.is_none())
        {
            return Err(UseDisposition::Rejected);
        }
    }
    let item_fence = command.item_fence.ok_or(UseDisposition::Rejected)?;
    match admission
        .root
        .replay_carried_quest_use(admission.character, item_fence, command_ref, intent.clone())
        .await
        .map_err(|_| UseDisposition::StaleState)?
    {
        Some(true) => {
            clear(admission, actor, command, UseDisposition::Committed).await;
            admission
                .refresh_quest_session(command.game_session_id, false)
                .await;
            return Ok(Some(UseDisposition::Committed));
        }
        Some(false) => return Err(UseDisposition::Rejected),
        None => {}
    }
    if let Some(pending) = pending {
        return Ok(Some(
            commit(admission, actor, command, pending, false).await,
        ));
    }
    let callbacks = super::super::quest_callbacks::callbacks();
    let Some(native) = admission
        .active_generation
        .and_then(ActiveGeneration::native_gameplay)
    else {
        return Ok(None);
    };
    let wanted = match source {
        UseSource::Definition(wanted) => {
            let runtime = admission.runtime.lock().await;
            let Some(index) = super::super::item_ref_admission::pinned_index(
                &runtime,
                admission.active_generation,
            ) else {
                return Ok(None);
            };
            let Some((key, revision)) = index.definition_at(*wanted) else {
                return Ok(None);
            };
            Some(TypedDefinitionRef {
                family: "Item".into(),
                production_key: key.into(),
                revision_ref: revision.into(),
            })
        }
        _ => None,
    };
    if wanted.as_ref().is_some_and(|wanted| {
        !callbacks.iter().any(|callback| {
            callback.trigger == trigger
                && callback
                    .item_on_target
                    .as_ref()
                    .is_some_and(|item| item.item == wanted.production_key)
        })
    }) {
        return Ok(None);
    }
    let inventory = admission
        .root
        .read_item_use_inventory(admission.character, admission.holder, item_fence)
        .await
        .map_err(|_| UseDisposition::Rejected)?;
    let instance = match source {
        UseSource::Handle(ItemKey::Instance(instance)) => *instance,
        UseSource::Definition(_) => {
            let definition = wanted.as_ref().ok_or(UseDisposition::NothingToUse)?;
            let runtime = admission.runtime.lock().await;
            let mut states = admission.states.spell_states.lock().await;
            let gate = states
                .item_uses
                .gate(&runtime, actor, command.game_session_id)
                .ok_or(UseDisposition::Rejected)?;
            inventory
                .first_unreserved(definition, &gate.unspendable)
                .map_err(|_| UseDisposition::NothingToUse)?
        }
        _ => return Ok(None),
    };
    let Some(entry) = inventory
        .backpack
        .as_ref()
        .into_iter()
        .flat_map(|bag| bag.entries.iter())
        .chain(inventory.nested.iter().map(|(_, entry)| entry))
        .find(|entry| entry.item.item_instance_id == instance)
    else {
        return Ok(None);
    };
    let candidates: Vec<_> = callbacks
        .iter()
        .filter(|callback| {
            callback.trigger == trigger
                && callback
                    .item_on_target
                    .as_ref()
                    .is_some_and(|item| item.item == entry.item.definition.production_key)
        })
        .collect();
    if candidates.is_empty() {
        return Ok(None);
    }
    let map = admission.bundle_map().ok_or(UseDisposition::Rejected)?;
    let target = if let Some(target) = target {
        let source = super::super::world_map::MapViewSource {
            overlay: &map.overlay,
            facts: &map.facts,
            content_generation: admission
                .active_generation
                .ok_or(UseDisposition::Rejected)?
                .identity()
                .client_artifact_digest(),
            reset_epoch: admission.reset_epoch,
        };
        let resolved = super::super::world_map::resolve_map_target(&source, target, |_| None::<()>)
            .map_err(|_| UseDisposition::StaleState)?;
        Some(CarriedTargetContext {
            position: Position {
                x: resolved.pos.x,
                y: resolved.pos.y,
                floor: resolved.pos.floor,
            },
            placement_key: resolved.placement_key,
            object_revision: target.expected_revision,
        })
    } else {
        None
    };
    // Ordinary consumables do not require quest admission. Only an authored
    // carried source enters the Character quest owner and its stronger fence.
    let fence = admission
        .current_quest_fence(command.game_session_id)
        .await
        .map_err(|_| UseDisposition::Rejected)?
        .ok_or(UseDisposition::Rejected)?;
    if item_fence != fence.item_fence() {
        return Err(UseDisposition::StaleState);
    }
    {
        let runtime = admission.runtime.lock().await;
        let mut states = admission.states.spell_states.lock().await;
        let gate = states
            .item_uses
            .gate(&runtime, actor, command.game_session_id)
            .ok_or(UseDisposition::Rejected)?;
        gate.admit(command.command_id, admission.owner_ms())?;
    }
    let weights = super::super::item_ref_admission::NativeItemWeights(native);
    weights
        .weight_of(&entry.item.definition)
        .map_err(|_| UseDisposition::NotPossible)?;
    let tracks = admission
        .states
        .quest_sessions
        .lock()
        .map_err(|_| UseDisposition::Rejected)?
        .get(&command.game_session_id)
        .and_then(|session| session.copy.as_ref())
        .ok_or(UseDisposition::Rejected)?
        .tracks()
        .clone();
    let mut eligible = candidates.into_iter().filter(|callback| {
        target.as_ref().is_none_or(|target| {
            super::super::quest_callbacks::matches(map, callback, target.placement_key)
        }) && callback.storage.iter().all(|storage| {
            let value = tracks
                .get(&storage.track)
                .copied()
                .or_else(|| {
                    admission
                        .quest_catalogue
                        .as_ref()?
                        .track(&storage.track)
                        .map(|track| track.initial)
                })
                .unwrap_or(-1);
            (storage.minimum..=storage.maximum).contains(&value)
        })
    });
    let callback = eligible.next().ok_or(UseDisposition::NotPossible)?;
    if eligible.next().is_some() {
        return Err(UseDisposition::Rejected);
    }
    if let Some(replacement) = callback
        .item_on_target
        .as_ref()
        .and_then(|item| item.replacement.as_ref())
    {
        weights
            .weight_of(&TypedDefinitionRef {
                family: "Item".into(),
                production_key: replacement.clone(),
                revision_ref: "definition-r1".into(),
            })
            .map_err(|_| UseDisposition::NotPossible)?;
    }
    let runtime = admission.runtime.lock().await;
    let expected = runtime
        .read_actor_position(actor)
        .map_err(|_| UseDisposition::Rejected)?;
    let local = expected.position();
    let actor_position = Position {
        x: local.x.try_into().map_err(|_| UseDisposition::Rejected)?,
        y: local.y.try_into().map_err(|_| UseDisposition::Rejected)?,
        floor: i8::try_from(local.floor)
            .map_err(|_| UseDisposition::Rejected)?
            .checked_neg()
            .ok_or(UseDisposition::Rejected)?,
    };
    if let Some(target) = &target {
        let at = crate::map::overlay::TilePos {
            x: actor_position.x,
            y: actor_position.y,
            floor: actor_position.floor,
        };
        let to = crate::map::overlay::TilePos {
            x: target.position.x,
            y: target.position.y,
            floor: target.position.floor,
        };
        if !super::super::map_door::reachable(at, to) {
            return Err(UseDisposition::TooFar);
        }
    }
    let recorded = admission
        .world_clock_utc_ms()
        .ok_or(UseDisposition::Rejected)?;
    let rule = callback
        .carried_use
        .as_ref()
        .ok_or(UseDisposition::Rejected)?;
    let minute = crate::foundation::world_time::tibian_minute(0, recorded)
        .ok_or(UseDisposition::Rejected)?;
    if !rule.actor_region.contains(actor_position) || !rule.time_window.contains(minute) {
        if let Some(message) = &callback.failure_message {
            admission
                .states
                .chat
                .push_quest_feedback(command.game_session_id, message);
        }
        return Err(UseDisposition::NotPossible);
    }
    let context = QuestWorldContext {
        projection_time_us: None,
        carried: Some(CarriedUseContext {
            target,
            source: instance,
            actor_position,
            recorded_utc_ms: recorded,
            intent: intent.clone(),
        }),
        map_revision: crate::map::overlay::map_revision(
            admission
                .bundle_map()
                .ok_or(UseDisposition::Rejected)?
                .overlay
                .base(),
        ),
        content_revision: format!(
            "sha256:{}",
            runtime
                .content_pin()
                .server_artifact_digest()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ),
        placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
        reset_epoch: admission.reset_epoch,
        actor_placement: actor.placement_identity(),
        movement_revision: expected.revision(),
    };
    drop(runtime);
    let pending = Pending {
        actor,
        command: command_ref,
        source: instance,
        intent,
        callback: callback.key.clone(),
        context,
        fence,
    };
    Ok(Some(commit(admission, actor, command, pending, true).await))
}

async fn commit(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    pending: Pending,
    fresh: bool,
) -> UseDisposition {
    let Some(catalogue) = admission.quest_catalogue.as_ref() else {
        return UseDisposition::Rejected;
    };
    // Match the existing Character writer order: revision slot before the owner lane.
    let mut slot = admission
        .revision_sequencer
        .acquire(pending.fence.character_id)
        .await;
    let Some(permit) = admission.spell_lane_permit().await else {
        return UseDisposition::StaleState;
    };
    let runtime = admission.runtime.lock().await;
    let mut states = admission.states.spell_states.lock().await;
    if !runtime
        .player_control_facts(actor, command.game_session_id)
        .is_ok_and(|facts| facts.control_loss.is_none())
        || states.is_dead(actor)
        || !states.spell_idle(&runtime, actor, command.game_session_id)
        || !admission
            .states
            .player_trades
            .physical(command.game_session_id, Some(pending.source))
    {
        drop(states);
        drop(runtime);
        clear(admission, actor, command, UseDisposition::Rejected).await;
        return UseDisposition::Rejected;
    }
    if fresh
        && !runtime
            .read_actor_position(actor)
            .is_ok_and(|position| position.revision() == pending.context.movement_revision)
    {
        return UseDisposition::StaleState;
    }
    if fresh {
        if let Some(target) = pending
            .context
            .carried
            .as_ref()
            .and_then(|carried| carried.target.as_ref())
            && !admission.bundle_map().is_some_and(|map| {
                map.facts.object_revision(target.placement_key) == target.object_revision
            })
        {
            return UseDisposition::StaleState;
        }
        let Some(gate) = states
            .item_uses
            .gate(&runtime, actor, command.game_session_id)
        else {
            return UseDisposition::Rejected;
        };
        let Ok(mut records) = admission.states.quest_callbacks.carried.lock() else {
            return UseDisposition::Rejected;
        };
        if records.contains_key(&command.game_session_id) {
            return UseDisposition::Exhausted;
        }
        records.insert(command.game_session_id, pending.clone());
        if !gate.unspendable.contains(&pending.source) {
            gate.unspendable.push(pending.source);
        }
        gate.in_flight_since_ms = Some(admission.owner_ms());
    }
    let result = commit_pending(admission, &mut slot, &permit, catalogue, &pending).await;
    #[cfg(test)]
    eprintln!(
        "CARRIED_WORLD_QUEST command={} callback={} result={result:?}",
        command.command_id, pending.callback
    );
    drop(states);
    drop(runtime);
    drop(slot);
    #[cfg(test)]
    if matches!(result, Ok(QuestTransitionOutcome::Committed(_)))
        && admission
            .states
            .quest_callbacks
            .interrupt_after_commit
            .swap(false, std::sync::atomic::Ordering::AcqRel)
    {
        return UseDisposition::StaleState;
    }
    match result {
        Ok(QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)) => {
            if admission
                .states
                .command_ingress
                .complete_item_use(pending.command, UseDisposition::Committed)
                .is_none()
            {
                return UseDisposition::StaleState;
            }
            clear(admission, actor, command, UseDisposition::Committed).await;
            admission
                .refresh_quest_session(command.game_session_id, false)
                .await;
            if let Some(callback) = super::super::quest_callbacks::callbacks()
                .iter()
                .find(|callback| callback.key == pending.callback)
                && let Some(message) = &callback.success_message
            {
                admission
                    .states
                    .chat
                    .push_quest_feedback(command.game_session_id, message);
            }
            UseDisposition::Committed
        }
        Err(crate::durability::character_progression::CharacterProgressionError::Unavailable(
            _,
        )) => UseDisposition::StaleState,
        _ => {
            if admission
                .states
                .command_ingress
                .complete_item_use(pending.command, UseDisposition::NotPossible)
                .is_none()
            {
                return UseDisposition::StaleState;
            }
            clear(admission, actor, command, UseDisposition::NotPossible).await;
            admission
                .refresh_quest_session(command.game_session_id, false)
                .await;
            UseDisposition::NotPossible
        }
    }
}

pub(in crate::gameplay_transport) async fn reconcile(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
) -> bool {
    let pending = match admission.states.quest_callbacks.carried.lock() {
        Ok(records) => records.get(&session).cloned(),
        Err(_) => return false,
    };
    let Some(pending) = pending else {
        return true;
    };
    let Some(catalogue) = admission.quest_catalogue.as_ref() else {
        return false;
    };
    let mut slot = admission
        .revision_sequencer
        .acquire(pending.fence.character_id)
        .await;
    let Some(permit) = admission.spell_lane_permit().await else {
        return false;
    };
    let outcome = commit_pending(admission, &mut slot, &permit, catalogue, &pending).await;
    drop(slot);
    let disposition = match outcome {
        Ok(QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)) => {
            UseDisposition::Committed
        }
        Err(crate::durability::character_progression::CharacterProgressionError::Unavailable(
            _,
        )) => return false,
        Ok(_) | Err(_) => UseDisposition::NotPossible,
    };
    if admission
        .states
        .command_ingress
        .complete_item_use(pending.command, disposition)
        .is_none()
    {
        return false;
    }
    clear(
        admission,
        pending.actor,
        UseCommand {
            game_session_id: session,
            command_id: pending.command.command_id().get(),
            item_fence: Some(pending.fence.item_fence()),
        },
        disposition,
    )
    .await;
    true
}

/// Pending native USE and cold reconciliation select the same atomic value writer.
async fn commit_pending(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    slot: &mut crate::durability::character_revision_sequencer::RevisionSlot,
    permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    catalogue: &std::sync::Arc<oteryn_gameplay_quests::quest::QuestStateCatalogue>,
    pending: &Pending,
) -> Result<
    QuestTransitionOutcome,
    crate::durability::character_progression::CharacterProgressionError,
> {
    let request = QuestTransitionRequest {
        transition_key: pending.callback.clone(),
        cause: QuestCause::Use(pending.command),
    };
    if super::super::quest_callbacks::callbacks()
        .iter()
        .find(|callback| callback.key == pending.callback)
        .is_some_and(|callback| callback.effects.item_reward.is_some())
    {
        let native = admission
            .active_generation
            .and_then(ActiveGeneration::native_gameplay)
            .ok_or(
                crate::durability::character_progression::CharacterProgressionError::InvalidInput,
            )?;
        let weights = crate::gameplay_transport::item_ref_admission::NativeItemWeights(native)
            .with_wheel(admission.premium_coordinator);
        slot.commit_world_reward(
            admission.root,
            admission.character,
            admission.holder,
            pending.fence,
            request,
            std::sync::Arc::clone(catalogue),
            permit,
            pending.context.clone(),
            crate::durability::reward_claim_mint::QuestRewardAdmission {
                native,
                weights: &weights,
                fee_payment: None,
                carried_use: None,
            },
        )
        .await
    } else {
        slot.commit_world_callback(
            admission.root,
            admission.character,
            admission.holder,
            pending.fence,
            request,
            std::sync::Arc::clone(catalogue),
            permit,
            pending.context.clone(),
        )
        .await
    }
}
