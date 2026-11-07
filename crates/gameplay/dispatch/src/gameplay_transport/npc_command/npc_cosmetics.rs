//! S06: the existing native NPC command owns the conversation; the fenced
//! source exchange independently resolves durable progress, costs and grants.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use crate::domain::CharacterId;
use crate::durability::npc_cosmetics::{NpcCosmeticOutcome, NpcCosmeticRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::npc_cosmetics as rules;

#[path = "npc_cosmetics/payment.rs"]
mod payment;

type Context = (u64, u64, String);
#[derive(Clone)]
pub(super) struct State {
    context: Context,
    actor: ExactActorRef,
    npc: EntityRef,
    topic: u16,
    effects: crate::durability::npc_cosmetics::NpcSourceEffects,
    pending: Option<(CharacterId, NpcCosmeticRequest, NpcTalkIntent)>,
}
pub(super) struct Step {
    state: State,
}
pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> Option<Step> {
    let context = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        runtime,
        actor,
        command.game_session_id,
        intent,
    )?;
    if !rules::supported(&context.2) {
        return None;
    }
    let text = intent.text.to_ascii_lowercase();
    if ["hi", "hello", "hail", "bye", "farewell", "goodbye"].contains(&text.as_str()) {
        return None;
    }
    let sessions = admission.states.npc_conversations.sessions.lock().ok()?;
    let previous = sessions.get(&command.game_session_id)?.cosmetics.as_ref();
    let state = previous
        .filter(|state| {
            state.context == context && state.actor == actor && state.npc == intent.npc_actor
        })
        .cloned()
        .unwrap_or(State {
            context,
            actor,
            npc: intent.npc_actor,
            topic: 0,
            effects: Default::default(),
            pending: None,
        });
    if !rules::recognizes(&state.context.2, &text, state.topic) {
        return None;
    }
    Some(Step { state })
}
pub(super) fn prepare_greet(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
    witness: super::fresh_greet::FreshNpcGreet,
) -> Option<Step> {
    let context = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        runtime,
        actor,
        command.game_session_id,
        intent,
    )?;
    if !rules::supported(&context.2) {
        return None;
    }
    let program = rules::source_program::embedded().ok()?;
    if !program
        .npc(&context.2)?
        .transitions
        .iter()
        .any(|transition| transition.trigger == rules::source_program::Trigger::Greet)
    {
        return None;
    }
    let greet = witness.into_source_context(
        &admission.states.npc_conversations,
        &admission.states.chat,
        runtime,
        actor,
        command,
        intent,
    )?;
    Some(Step {
        state: State {
            context,
            actor,
            npc: intent.npc_actor,
            topic: 0,
            pending: None,
            effects: crate::durability::npc_cosmetics::NpcSourceEffects {
                greet: Some(greet),
                ..Default::default()
            },
        },
    })
}

fn response(intent: &NpcTalkIntent, outcome: NpcCosmeticOutcome) -> NpcTalkOutcome {
    NpcTalkOutcome {
        unknown: false,
        inventory_changed: outcome.inventory_changed,
        disposition: NpcIntentDisposition::Ok,
        conversation: (!outcome.reply.is_empty() || !outcome.reply_lines.is_empty()).then_some(
            NpcConversation {
                open: (!outcome.close).then_some((intent.npc_actor, NpcConversationState::Open)),
                lines: if outcome.reply_lines.is_empty() {
                    vec![outcome.reply]
                } else {
                    outcome.reply_lines
                },
            },
        ),
        window: None,
    }
}
fn install(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    state: Option<State>,
) {
    if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
        && let Some(session) = sessions.get_mut(&session)
    {
        session.cosmetics = state;
    }
}
// The existing equipment owner is also the authority after an NPC removes a
// carried recipe operand from a worn slot. Callers retain the spell lane.
pub(super) async fn refresh_equipment(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
) -> bool {
    admission
        .states
        .attack
        .lock()
        .await
        .forget_weapon(actor, session);
    if !admission
        .checkpoint_timed_equipment(actor, session, true)
        .await
    {
        #[cfg(test)]
        eprintln!("SRV7_NPC_REFRESH_UNAVAILABLE stage=checkpoint_timed_equipment");
        return false;
    }
    let Ok(effects) = admission.read_admission_effects(session).await else {
        #[cfg(test)]
        eprintln!("SRV7_NPC_REFRESH_UNAVAILABLE stage=read_admission_effects");
        return false;
    };
    let runtime = admission.runtime.lock().await;
    let installed = admission
        .install_admission_suppressions(&runtime, actor, session, effects)
        .await
        .is_some();
    #[cfg(test)]
    if !installed {
        eprintln!("SRV7_NPC_REFRESH_UNAVAILABLE stage=install_admission_suppressions");
    }
    installed
}

pub(super) async fn apply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcTalkIntent,
    step: Step,
) -> Option<NpcTalkOutcome> {
    let rejected = || Some(NpcTalkOutcome::only(NpcIntentDisposition::Rejected));
    let unknown = || {
        Some(NpcTalkOutcome {
            unknown: true,
            ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
        })
    };
    let mut state = step.state;
    if state.pending.is_some() {
        return unknown();
    }
    state.effects.quest_catalogue = admission.quest_catalogue.clone();
    let (Some(item_fence), Ok(id), Some(native)) = (
        command.item_fence,
        CommandId::new(command.command_id),
        admission
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay),
    ) else {
        return rejected();
    };
    let Some(permit) = admission.spell_lane_permit().await else {
        return unknown();
    };
    let mut slot = admission
        .revision_sequencer
        .acquire(item_fence.character_id)
        .await;
    let Ok(Some(fence)) = admission.current_quest_fence(command.game_session_id).await else {
        return unknown();
    };
    if fence.item_fence() != item_fence {
        return rejected();
    }
    {
        let runtime = admission.runtime.lock().await;
        let Ok(states) = admission.states.spell_states.try_lock() else {
            return rejected();
        };
        if states.is_dead(actor)
            || !runtime
                .player_control_facts(actor, command.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
            || !states.spell_idle(&runtime, actor, command.game_session_id)
            || admission.states.npc_conversations.current_npc(
                &admission.states.chat,
                &runtime,
                actor,
                command.game_session_id,
                &intent,
            ) != Some(state.context.clone())
            || !runtime
                .bound_attacker_lease(actor, command.game_session_id)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
        {
            return rejected();
        }
    }
    let request = NpcCosmeticRequest {
        command: CommandRef::new(command.game_session_id, id),
        npc: state.context.2.clone(),
        message: intent.text.clone(),
        topic: state.topic,
    };
    if matches!(
        request.npc.as_str(),
        "oteryn:npc.king_tibianus"
            | "oteryn:npc.queen_eloise"
            | "oteryn:npc.emperor_kruzak"
            | "oteryn:npc.irmana"
    ) {
        let Ok(payment) = payment::prepare(admission, command, fence).await else {
            return rejected();
        };
        state.effects.payment = Some(payment);
    }
    state.pending = Some((fence.character_id, request.clone(), intent.clone()));
    install(admission, command.game_session_id, Some(state.clone()));
    let source_program = rules::source_program::supported(&state.context.2);
    let premium = match (
        admission.premium_coordinator,
        admission.session_account(command.game_session_id),
    ) {
        (Some(coordinator), Some(account)) => {
            let _ = coordinator.premium_gate(account).await;
            coordinator.view(account)
        }
        _ => std::sync::Arc::new(
            super::super::spell_premium_coordinator::RegisteredSpellAccessOwner::unavailable(),
        ),
    };
    let outcome = if source_program {
        admission
            .commit_npc_source_actor_owned(
                actor,
                super::super::npc_source_actor_effect::NpcSourceCall {
                    fence,
                    request,
                    permit: &permit,
                    native,
                    premium: Some(premium.as_ref()),
                },
                &mut state.effects,
            )
            .await
    } else {
        admission
            .root
            .commit_npc_cosmetic(
                admission.character,
                admission.holder,
                fence,
                request,
                &permit,
                native,
            )
            .await
            .map(Some)
    };
    // Unknown commit retries must retain the first complete condition observation.
    if state.pending.is_some() && state.effects.actor_observation.is_some() {
        install(admission, command.game_session_id, Some(state.clone()));
    }
    slot.observe_endgame_progression();
    drop(slot);
    match outcome {
        Ok(None) => {
            install(admission, command.game_session_id, None);
            None
        }
        Ok(Some(outcome)) => {
            #[cfg(test)]
            if state.context.2 == "oteryn:npc.chondur" {
                eprintln!(
                    "SRV7_CHONDUR_KNOWN_OUTCOME fresh={} inventory_changed={} topic={}",
                    outcome.fresh, outcome.inventory_changed, outcome.topic
                );
            }
            publish_source_effect(admission, actor, command, &intent, &state, &outcome).await;
            if outcome.inventory_changed
                && !refresh_equipment(admission, actor, command.game_session_id).await
            {
                return unknown();
            }
            state.pending = None;
            state.effects = Default::default();
            state.topic = outcome.topic;
            install(
                admission,
                command.game_session_id,
                (!outcome.close).then_some(state),
            );
            if outcome.close
                && let Some(npc) = admission.states.chat.engaged(command.game_session_id)
            {
                admission
                    .states
                    .chat
                    .end_engagement(command.game_session_id, npc);
            }
            admission
                .refresh_quest_session(command.game_session_id, false)
                .await;
            Some(response(&intent, outcome))
        }
        Err(crate::durability::character_progression::CharacterProgressionError::Unavailable(
            error,
        )) => {
            #[cfg(test)]
            {
                use crate::durability::DurabilityError as E;
                let kind = match &error {
                    E::InvalidStoredState => "invalid_stored_state",
                    E::RootPassDeadlineExceeded => "deadline",
                    E::RootTaskFailed => "root_task_failed",
                    E::Database(_) => "database",
                    E::CommitRejected => "commit_rejected",
                    E::CommitOutcomeUnknown => "commit_unknown",
                    _ => "other",
                };
                eprintln!("SRV7_NPC_UNAVAILABLE kind={kind}");
            }
            #[cfg(not(test))]
            let _ = error;
            unknown()
        }
        _ => {
            install(admission, command.game_session_id, None);
            rejected()
        }
    }
}
pub(in crate::gameplay_transport) async fn resume(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
) -> Option<NpcTalkOutcome> {
    let mut state = admission
        .states
        .npc_conversations
        .sessions
        .lock()
        .ok()?
        .get(&command.game_session_id)?
        .cosmetics
        .clone()?;
    let (character, request, intent) = state.pending.clone()?;
    if request.command
        != CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).ok()?,
        )
    {
        return None;
    }
    let outcome = if state.effects.greet.is_some()
        && crate::durability::npc_cosmetics::has_readonly_source_greet(&request.npc)
    {
        // A readonly source intentionally has no gameplay Parent receipt. Re-enter
        // its original occurrence under current live owners and the retained body.
        let Some(permit) = admission.spell_lane_permit().await else {
            return Some(NpcTalkOutcome {
                unknown: true,
                ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
            });
        };
        let Some(native) = admission
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
        else {
            return Some(NpcTalkOutcome {
                unknown: true,
                ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
            });
        };
        let mut slot = admission.revision_sequencer.acquire(character).await;
        let Ok(Some(fence)) = admission.current_quest_fence(command.game_session_id).await else {
            return Some(NpcTalkOutcome {
                unknown: true,
                ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
            });
        };
        if fence.character_id != character {
            return Some(NpcTalkOutcome::only(NpcIntentDisposition::Rejected));
        }
        let premium = match (
            admission.premium_coordinator,
            admission.session_account(command.game_session_id),
        ) {
            (Some(coordinator), Some(account)) => {
                let _ = coordinator.premium_gate(account).await;
                coordinator.view(account)
            }
            _ => std::sync::Arc::new(
                super::super::spell_premium_coordinator::RegisteredSpellAccessOwner::unavailable(),
            ),
        };
        let result = admission
            .commit_npc_source_actor_owned(
                actor,
                super::super::npc_source_actor_effect::NpcSourceCall {
                    fence,
                    request,
                    permit: &permit,
                    native,
                    premium: Some(premium.as_ref()),
                },
                &mut state.effects,
            )
            .await;
        install(admission, command.game_session_id, Some(state.clone()));
        slot.observe_endgame_progression();
        drop(slot);
        drop(permit);
        result
    } else {
        admission
            .root
            .reconcile_npc_cosmetic(admission.character, character, request)
            .await
    };
    if outcome.is_ok()
        && admission
            .resolve_pending_npc_ajax_effect(actor, command.game_session_id)
            .await
            .is_err()
    {
        return Some(NpcTalkOutcome {
            unknown: true,
            ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
        });
    }
    match outcome {
        Ok(Some(outcome)) => {
            if outcome.inventory_changed {
                let Some(_permit) = admission.spell_lane_permit().await else {
                    return Some(NpcTalkOutcome {
                        unknown: true,
                        ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
                    });
                };
                if !refresh_equipment(admission, actor, command.game_session_id).await {
                    return Some(NpcTalkOutcome {
                        unknown: true,
                        ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
                    });
                }
            }
            let runtime = admission.runtime.lock().await;
            let current = admission.states.npc_conversations.current_npc(
                &admission.states.chat,
                &runtime,
                state.actor,
                command.game_session_id,
                &intent,
            ) == Some(state.context.clone());
            drop(runtime);
            state.pending = None;
            state.effects = Default::default();
            state.topic = outcome.topic;
            if current
                && outcome.close
                && let Some(npc) = admission.states.chat.engaged(command.game_session_id)
            {
                admission
                    .states
                    .chat
                    .end_engagement(command.game_session_id, npc);
            }
            install(
                admission,
                command.game_session_id,
                (current && !outcome.close).then_some(state),
            );
            admission
                .refresh_quest_session(command.game_session_id, false)
                .await;
            Some(response(&intent, outcome))
        }
        Ok(None) => {
            install(admission, command.game_session_id, None);
            Some(NpcTalkOutcome::only(NpcIntentDisposition::Rejected))
        }
        Err(_) => Some(NpcTalkOutcome {
            unknown: true,
            ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
        }),
    }
}

// The source emits at the actual player, once after a known fresh commit. A replay
// carries no transient witness; a lost scope/control cannot publish into another view.
async fn publish_source_effect(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    state: &State,
    outcome: &NpcCosmeticOutcome,
) {
    if !outcome.fresh || !outcome.committed {
        return;
    }
    if outcome.magic_effect.is_none() && outcome.advance_notices.is_empty() {
        return;
    }
    let Some(fence) = command.item_fence else {
        return;
    };
    let runtime = admission.runtime.lock().await;
    if runtime.owner_fence().is_err()
        || runtime.binding().runtime_scope() != fence.runtime_scope
        || runtime.binding().scope_generation() != fence.scope_ownership_generation
        || !runtime
            .player_control_facts(actor, command.game_session_id)
            .is_ok_and(|facts| facts.control_loss.is_none())
        || !runtime
            .bound_attacker_lease(actor, command.game_session_id)
            .is_ok_and(|lease| {
                lease.character_id().as_bytes() == fence.character_id.as_bytes()
                    && lease.generation() == fence.character_lease_generation
            })
        || admission.states.npc_conversations.current_npc(
            &admission.states.chat,
            &runtime,
            actor,
            command.game_session_id,
            intent,
        ) != Some(state.context.clone())
    {
        return;
    }
    let Ok(position) = runtime.read_actor_position(actor) else {
        return;
    };
    let Ok(mut states) = admission.states.spell_states.try_lock() else {
        return;
    };
    if states.is_dead(actor) {
        return;
    }
    for notice in &outcome.advance_notices {
        admission.states.server_messages.npc_source_advance(
            command.game_session_id,
            fence.connection_generation.get(),
            notice,
        );
    }
    if let Some(cue) = outcome.magic_effect {
        states
            .sounds
            .record_item_effect(&runtime, actor, position.position(), cue, false);
    }
}
