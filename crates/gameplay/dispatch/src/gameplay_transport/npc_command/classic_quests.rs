//! Batch-2 NPC branches on the existing conversation and fenced QuestState writer.
//! Literal text is content; a source reply never acts as evidence that its mutation committed.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use crate::domain::CharacterId;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::{classic_dialogue, classic_talk};

type Context = (u64, u64, String);

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Conversation {
    context: Context,
    actor: ExactActorRef,
    npc: EntityRef,
    topic: classic_talk::Topic,
}

#[derive(Clone)]
pub(super) enum Pending {
    Topic(Conversation),
    Commit {
        character: CharacterId,
        request: QuestTransitionRequest,
        intent: NpcTalkIntent,
        reply: Vec<String>,
        next: Option<Conversation>,
    },
}

pub(super) struct CommitStep {
    context: Context,
    previous: Option<Conversation>,
    next: Option<Conversation>,
    transition: &'static str,
    reply: Vec<String>,
}

pub(super) enum Step {
    Reply(Vec<String>),
    Refused,
    Commit(Box<CommitStep>),
    Held,
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
    if ![
        classic_talk::JEROM,
        classic_talk::QUEEN,
        classic_talk::ROTTIN,
    ]
    .contains(&context.2.as_str())
    {
        return None;
    }
    let tracks = {
        let sessions = admission.states.quest_sessions.lock().ok()?;
        sessions
            .get(&command.game_session_id)?
            .copy
            .as_ref()?
            .read_tracks(admission.quest_catalogue.as_deref()?)?
    };
    let mut sessions = admission.states.npc_conversations.sessions.lock().ok()?;
    let state = sessions.get_mut(&command.game_session_id)?;
    if matches!(state.classic.as_ref(), Some(Pending::Commit { .. })) {
        return Some(Step::Held);
    }
    let previous = match state.classic.as_ref() {
        Some(Pending::Topic(previous))
            if previous.context == context
                && previous.actor == actor
                && previous.npc == intent.npc_actor =>
        {
            Some(previous.clone())
        }
        _ => None,
    };
    if previous.is_none() {
        state.classic = None;
    }
    let choice = classic_talk::choose(
        &context.2,
        &intent.text,
        previous.as_ref().map(|p| p.topic),
        &tracks,
    )?;
    if choice.deferred {
        return Some(Step::Refused);
    }
    let reply = choice
        .branches
        .iter()
        .map(|key| classic_dialogue::reply(&context.2, key).map(<[String]>::to_vec))
        .collect::<Option<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();
    let next = choice.topic.map(|topic| Conversation {
        context: context.clone(),
        actor,
        npc: intent.npc_actor,
        topic,
    });
    if let Some(transition) = choice.transition {
        Some(Step::Commit(Box::new(CommitStep {
            context,
            previous,
            next,
            transition,
            reply,
        })))
    } else {
        state.classic = next.map(Pending::Topic);
        Some(Step::Reply(reply))
    }
}

fn response(intent: &NpcTalkIntent, lines: Vec<String>) -> NpcTalkOutcome {
    NpcTalkOutcome {
        unknown: false,
        inventory_changed: false,
        disposition: NpcIntentDisposition::Ok,
        conversation: Some(NpcConversation {
            open: Some((intent.npc_actor, NpcConversationState::Open)),
            lines,
        }),
        window: None,
    }
}

fn install_topic(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    next: Option<Conversation>,
) {
    if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
        && let Some(state) = sessions.get_mut(&session)
    {
        state.classic = next.map(Pending::Topic);
    }
}

pub(super) async fn apply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcTalkIntent,
    step: Step,
) -> NpcTalkOutcome {
    let rejected = || NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    let unknown = || NpcTalkOutcome {
        unknown: true,
        ..rejected()
    };
    let CommitStep {
        context,
        previous,
        next,
        transition,
        reply,
    } = match step {
        Step::Reply(lines) => return response(&intent, lines),
        Step::Refused => return rejected(),
        Step::Held => return unknown(),
        Step::Commit(commit) => *commit,
    };
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
        let Ok(states) = admission.states.spell_states.try_lock() else {
            return rejected();
        };
        if states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, command.game_session_id)
            || admission.states.npc_conversations.current_npc(
                &admission.states.chat,
                &runtime,
                actor,
                command.game_session_id,
                &intent,
            ) != Some(context)
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
    let request = QuestTransitionRequest {
        transition_key: transition.into(),
        cause: QuestCause::Command(CommandRef::new(command.game_session_id, id)),
    };
    {
        let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock() else {
            return rejected();
        };
        let Some(state) = sessions.get_mut(&command.game_session_id) else {
            return rejected();
        };
        if matches!(state.classic.as_ref(), Some(Pending::Commit { .. })) {
            return unknown();
        }
        let current = match state.classic.as_ref() {
            Some(Pending::Topic(topic)) => Some(topic),
            _ => None,
        };
        if current != previous.as_ref() {
            return rejected();
        }
        state.classic = Some(Pending::Commit {
            character: fence.character_id,
            request: request.clone(),
            intent: intent.clone(),
            reply: reply.clone(),
            next: next.clone(),
        });
    }
    let outcome = slot
        .commit_quest_transition(
            admission.root,
            admission.character,
            admission.holder,
            fence,
            request,
            std::sync::Arc::clone(catalogue),
        )
        .await;
    drop(slot);
    match outcome {
        Ok(QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)) => {
            install_topic(admission, command.game_session_id, next);
            admission
                .refresh_quest_session(command.game_session_id, false)
                .await;
            response(&intent, reply)
        }
        Err(crate::durability::character_progression::CharacterProgressionError::Unavailable(
            _,
        )) => unknown(),
        _ => {
            install_topic(admission, command.game_session_id, None);
            rejected()
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
        .classic
        .clone()?;
    let Pending::Commit {
        character,
        request,
        intent,
        reply,
        next,
    } = pending
    else {
        return None;
    };
    let id = CommandId::new(command.command_id).ok()?;
    if request.cause != QuestCause::Command(CommandRef::new(command.game_session_id, id)) {
        return None;
    }
    match admission
        .root
        .reconcile_character_quest_transition(admission.character, character, request)
        .await
    {
        Ok(Some(_)) => {
            install_topic(admission, command.game_session_id, next);
            admission
                .refresh_quest_session(command.game_session_id, false)
                .await;
            Some(response(&intent, reply))
        }
        Ok(None) => {
            install_topic(admission, command.game_session_id, None);
            Some(NpcTalkOutcome::only(NpcIntentDisposition::Rejected))
        }
        Err(_) => Some(NpcTalkOutcome {
            unknown: true,
            ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
        }),
    }
}
