//! Targuna uses command 7, the existing conversation generation and QuestState.
//! Topics are bound to the exact visible NPC and never carried across a new conversation.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use crate::domain::CharacterId;
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::targuna as quest;
use quest::npc_rules::{self, EmilianaPlace, Mutation, Npc, NpcStep, Reply, Topic};

type Context = (u64, u64, String);

#[derive(Clone)]
pub(super) enum Pending {
    Topic {
        context: Context,
        npc: EntityRef,
        topic: Option<Topic>,
    },
    Commit {
        actor: ExactActorRef,
        character: CharacterId,
        request: QuestTransitionRequest,
        intent: NpcTalkIntent,
        context: Context,
        next_topic: Option<Topic>,
        lines: Vec<String>,
    },
}

pub(super) struct Step {
    context: Context,
    npc: Npc,
    topic: Option<Topic>,
    first: bool,
    place: EmilianaPlace,
    held: bool,
}

fn identity(key: &str) -> Option<Npc> {
    match key {
        "oteryn:npc.lizzie" => Some(Npc::Lizzie),
        "oteryn:npc.camilla" => Some(Npc::Camilla),
        "oteryn:npc.sterling" => Some(Npc::Sterling),
        "oteryn:npc.morla" => Some(Npc::Morla),
        "oteryn:npc.emiliana" => Some(Npc::Emiliana),
        "oteryn:npc.adrian" => Some(Npc::Adrian),
        _ => None,
    }
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
    let npc = identity(&context.2)?;
    // Adrian's actual bank commands must continue to the existing bank owner.
    if npc == Npc::Adrian
        && quest::dialogue::keyword(npc, &intent.text).is_none()
        && !matches!(intent.text.to_ascii_lowercase().as_str(), "hi" | "hello")
    {
        return None;
    }
    let live =
        service_context::live_npc(runtime, actor, command.game_session_id, intent.npc_actor)?;
    let speaker = service_context::speaker_position(runtime, actor, command.game_session_id)?;
    // Runtime movement floors are source z; the map overlay uses its negation.
    let place = npc_rules::emiliana_place(live.position.floor, speaker.x, speaker.y, speaker.floor);
    let sessions = admission.states.npc_conversations.sessions.lock().ok()?;
    let pending = sessions
        .get(&command.game_session_id)
        .and_then(|state| state.targuna.as_ref());
    let (topic, first, held) = match pending {
        Some(Pending::Topic {
            context: previous,
            npc,
            topic,
        }) if previous == &context && npc == &intent.npc_actor => (*topic, false, false),
        Some(Pending::Commit { .. }) => (None, false, true),
        _ => (None, true, false),
    };
    Some(Step {
        context,
        npc,
        topic,
        first,
        place,
        held,
    })
}

fn reply(intent: &NpcTalkIntent, lines: Vec<String>, inventory_changed: bool) -> NpcTalkOutcome {
    NpcTalkOutcome {
        unknown: false,
        inventory_changed,
        disposition: NpcIntentDisposition::Ok,
        conversation: Some(NpcConversation {
            open: Some((intent.npc_actor, NpcConversationState::Open)),
            lines,
        }),
        window: None,
    }
}

/// A known receipt stays successful after its conversation has closed. The
/// retained reply may only target the same catalogue/conversation/NPC context.
fn publish_current_reply(
    current: Option<Context>,
    expected: &Context,
    rendered: NpcTalkOutcome,
) -> NpcTalkOutcome {
    if current.as_ref() == Some(expected) {
        rendered
    } else {
        NpcTalkOutcome::only(NpcIntentDisposition::Ok)
    }
}

async fn publish_reply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    intent: &NpcTalkIntent,
    context: &Context,
    lines: Vec<String>,
) -> NpcTalkOutcome {
    // Re-read after every awaited commit/reconciliation and live-state refresh.
    // This proof is presentation-only: never undo a receipt or reset a new owner.
    let runtime = admission.runtime.lock().await;
    publish_current_reply(
        admission.states.npc_conversations.current_npc(
            &admission.states.chat,
            &runtime,
            actor,
            session,
            intent,
        ),
        context,
        reply(intent, lines, false),
    )
}

fn topic(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    context: Context,
    npc: EntityRef,
    next: Option<Topic>,
) -> bool {
    let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock() else {
        return false;
    };
    sessions.entry(session).or_default().targuna = Some(Pending::Topic {
        context,
        npc,
        topic: next,
    });
    true
}

fn clear(admission: &ComposedFreshAdmission<'_, '_, '_>, session: GameSessionId) {
    if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
        && let Some(state) = sessions.get_mut(&session)
    {
        state.targuna = None;
    }
}

pub(super) async fn apply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcTalkIntent,
    step: Step,
    ordinary: NpcTalkOutcome,
) -> NpcTalkOutcome {
    let rejected = || NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    let unknown = || NpcTalkOutcome {
        unknown: true,
        ..rejected()
    };
    if step.held {
        return unknown();
    }
    let (Some(item_fence), Ok(id), Some(catalogue)) = (
        command.item_fence,
        CommandId::new(command.command_id),
        admission.quest_catalogue.as_ref(),
    ) else {
        return rejected();
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
        let states = admission.states.spell_states.lock().await;
        if states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, command.game_session_id)
            || !runtime
                .player_control_facts(actor, command.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
            || admission.states.npc_conversations.current_npc(
                &admission.states.chat,
                &runtime,
                actor,
                command.game_session_id,
                &intent,
            ) != Some(step.context.clone())
            || !runtime
                .bound_attacker_lease(actor, command.game_session_id)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
        {
            return rejected();
        }
        let Some(live) =
            service_context::live_npc(&runtime, actor, command.game_session_id, intent.npc_actor)
        else {
            return rejected();
        };
        let Some(speaker) =
            service_context::speaker_position(&runtime, actor, command.game_session_id)
        else {
            return rejected();
        };
        if npc_rules::emiliana_place(live.position.floor, speaker.x, speaker.y, speaker.floor)
            != step.place
        {
            return rejected();
        }
    }
    let level = match admission
        .root
        .read_character_progression(admission.character, fence.character_id)
        .await
    {
        Ok(Some(state)) if state.character_revision == fence.expected_character_revision => {
            state.level
        }
        _ => return unknown(),
    };
    let copy = match admission
        .root
        .read_character_quest_state(admission.character, fence.character_id)
        .await
    {
        Ok(copy) if copy.character_revision() == Some(fence.expected_character_revision) => copy,
        _ => return unknown(),
    };
    let source_state = quest::npc_state(copy.tracks());
    // The first command7 can follow an earlier public "hi". Only replace a
    // greeting that the existing talk owner actually emitted for this command;
    // otherwise its first mission keyword must run immediately.
    let opening = step.first
        && ordinary.conversation.as_ref().is_some_and(|conversation| {
            quest::dialogue::is_opening_greeting(step.npc, &conversation.lines)
        });
    let selected = if opening {
        Some(NpcStep {
            topic: None,
            mutation: None,
            reply: Reply {
                source_line: npc_rules::greet(step.npc, source_state, step.place),
                numbers: Vec::new(),
            },
        })
    } else if let Some(reply) = quest::dialogue::keyword(step.npc, &intent.text) {
        Some(NpcStep {
            topic: step.topic,
            mutation: None,
            reply,
        })
    } else {
        npc_rules::choose(
            step.npc,
            &intent.text,
            step.topic,
            source_state,
            level,
            step.place,
        )
    };
    let Some(selected) = selected else {
        topic(
            admission,
            command.game_session_id,
            step.context,
            intent.npc_actor,
            step.topic,
        );
        return ordinary;
    };
    let Some(mut lines) = quest::dialogue::render(step.npc, &selected.reply) else {
        return rejected();
    };
    if lines.iter().any(|line| line.contains("|PLAYERNAME|")) {
        let Ok(Some(name)) = admission
            .root
            .read_character_chat_name(admission.character, fence.character_id)
            .await
        else {
            return unknown();
        };
        for line in &mut lines {
            *line = line.replace("|PLAYERNAME|", name.as_str());
        }
    }
    let Some(mutation) = selected.mutation else {
        if !topic(
            admission,
            command.game_session_id,
            step.context.clone(),
            intent.npc_actor,
            selected.topic,
        ) {
            return rejected();
        }
        return publish_reply(
            admission,
            actor,
            command.game_session_id,
            &intent,
            &step.context,
            lines,
        )
        .await;
    };
    // No partial reward, success reply, XP, money debit or quest flag without the
    // approved atomic owner. Coordinator leaves that shared contract to another PR:
    // https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728 (4,5).
    if matches!(
        mutation,
        Mutation::MorlaReward | Mutation::TurnipsReward | Mutation::ReplaceTurnips
    ) {
        return rejected();
    }
    let Some(transition) = quest::mutation_transition(mutation, source_state) else {
        return rejected();
    };
    let request = QuestTransitionRequest {
        transition_key: transition,
        cause: QuestCause::Command(CommandRef::new(command.game_session_id, id)),
    };
    {
        let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock() else {
            return rejected();
        };
        let state = sessions.entry(command.game_session_id).or_default();
        if matches!(state.targuna, Some(Pending::Commit { .. })) {
            return unknown();
        }
        state.targuna = Some(Pending::Commit {
            actor,
            character: fence.character_id,
            request: request.clone(),
            intent: intent.clone(),
            context: step.context.clone(),
            next_topic: selected.topic,
            lines: lines.clone(),
        });
    }
    let committed = match slot
        .commit_quest_transition(
            admission.root,
            admission.character,
            admission.holder,
            fence,
            request,
            catalogue.clone(),
        )
        .await
    {
        Ok(QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)) => {
            Some(true)
        }
        Err(CharacterProgressionError::Unavailable(_)) => None,
        _ => Some(false),
    };
    // Admission may consume the transition's XP obligation through this same sequencer.
    drop(slot);
    match committed {
        None => unknown(),
        Some(false) => {
            topic(
                admission,
                command.game_session_id,
                step.context,
                intent.npc_actor,
                step.topic,
            );
            rejected()
        }
        Some(true) => {
            topic(
                admission,
                command.game_session_id,
                step.context.clone(),
                intent.npc_actor,
                selected.topic,
            );
            admission.states.loop_ground.invalidate();
            admission
                .refresh_quest_session(command.game_session_id, false)
                .await;
            admission
                .refresh_level_up_vitals(actor, command.game_session_id)
                .await;
            publish_reply(
                admission,
                actor,
                command.game_session_id,
                &intent,
                &step.context,
                lines,
            )
            .await
        }
    }
}

pub(in crate::gameplay_transport) async fn resume(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
) -> Option<NpcTalkOutcome> {
    let pending = admission
        .states
        .npc_conversations
        .sessions
        .lock()
        .ok()?
        .get(&command.game_session_id)?
        .targuna
        .clone()?;
    let Pending::Commit {
        actor,
        character,
        request,
        intent,
        context,
        next_topic,
        lines,
    } = pending
    else {
        return None;
    };
    let reference = CommandRef::new(
        command.game_session_id,
        CommandId::new(command.command_id).ok()?,
    );
    if request.cause != QuestCause::Command(reference) {
        return None;
    }
    let committed = admission
        .root
        .reconcile_character_quest_transition(admission.character, character, request)
        .await
        .map(|receipt| receipt.is_some())
        .map_err(|_| ());
    Some(match committed {
        Ok(true) => {
            topic(
                admission,
                command.game_session_id,
                context.clone(),
                intent.npc_actor,
                next_topic,
            );
            admission.states.loop_ground.invalidate();
            admission
                .refresh_quest_session(command.game_session_id, false)
                .await;
            admission
                .refresh_level_up_vitals(actor, command.game_session_id)
                .await;
            publish_reply(
                admission,
                actor,
                command.game_session_id,
                &intent,
                &context,
                lines,
            )
            .await
        }
        Ok(false) => {
            clear(admission, command.game_session_id);
            NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
        }
        Err(()) => NpcTalkOutcome {
            unknown: true,
            ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
        },
    })
}

#[cfg(test)]
mod publication_tests;
