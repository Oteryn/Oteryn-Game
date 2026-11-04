//! Liquid Black and Jack Springer use the existing NPC conversation, fenced
//! QuestState writer and command receipt. The two closed source policies do not
//! accept a client storage value, transition key or NPC proof.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use crate::domain::CharacterId;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::grave_danger as grave;
use oteryn_gameplay_quests::quest::liquid_black as quest;

type Context = (u64, u64, String);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Topic {
    Liquid(quest::Topic),
    Grave(grave::Topic),
}

#[derive(Clone, Copy)]
pub(super) enum Reply {
    Liquid(quest::Reply),
    Grave(grave::Reply),
    GraveGreeting { started: bool },
}

impl Reply {
    fn lines(self) -> Option<&'static [String]> {
        match self {
            Self::Liquid(key) => quest::reply(key),
            Self::Grave(key) => grave::reply(key),
            Self::GraveGreeting { started } => grave::greeting(i64::from(started)),
        }
    }
}

struct Choice {
    reply: Option<Reply>,
    transition: Option<&'static str>,
    topic: Option<Topic>,
    deferred: bool,
}

#[derive(Clone)]
pub(super) enum State {
    Topic {
        context: Context,
        npc: EntityRef,
        topic: Topic,
    },
    Commit {
        character: CharacterId,
        context: Context,
        request: QuestTransitionRequest,
        intent: NpcTalkIntent,
        reply: Option<Reply>,
        next_topic: Option<Topic>,
    },
}

impl State {
    fn topic(&self, current: &Context, actor: EntityRef) -> Option<Topic> {
        match self {
            Self::Topic {
                context,
                npc,
                topic,
            } if context == current && *npc == actor => Some(*topic),
            _ => None,
        }
    }
}

pub(super) struct Step {
    context: Context,
    choice: Option<Choice>,
}

fn is_grave_greeting(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
    intent: &NpcTalkIntent,
    ordinary: &NpcTalkOutcome,
) -> bool {
    ordinary.disposition == NpcIntentDisposition::Ok
        && ordinary.conversation.as_ref().is_some_and(|conversation| {
            conversation.open == Some((intent.npc_actor, NpcConversationState::Open))
                && grave::greeting(0).is_some_and(|source| {
                    conversation.lines == source
                        || admission
                            .states
                            .chat
                            .speaker_name(command.game_session_id)
                            .is_some_and(|name| {
                                conversation.lines
                                    == source
                                        .iter()
                                        .map(|line| line.replace("|PLAYERNAME|", &name))
                                        .collect::<Vec<_>>()
                            })
                })
        })
}

fn refuse_grave_unavailable(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
    ordinary: &NpcTalkOutcome,
    context: Context,
) -> Step {
    // Crystal jack_springer.lua:61-70 requires QuestLine before selecting either
    // greeting. Undo only this unseen fresh opening under the caller's runtime
    // lock. Quest/session mutexes are already released; retained commits never
    // enter this branch and keep their exact recovery request.
    if is_grave_greeting(admission, command, intent, ordinary)
        && let Some(npc) =
            service_context::live_npc(runtime, actor, command.game_session_id, intent.npc_actor)
    {
        admission
            .states
            .chat
            .end_engagement(command.game_session_id, npc.actor);
    }
    Step {
        context,
        choice: Some(Choice {
            reply: None,
            transition: None,
            topic: None,
            deferred: true,
        }),
    }
}

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
    ordinary: &NpcTalkOutcome,
) -> Option<Step> {
    let context = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        runtime,
        actor,
        command.game_session_id,
        intent,
    )?;
    if quest::Npc::from_key(&context.2).is_none() && context.2 != grave::NPC {
        return None;
    }
    let topic = {
        let sessions = admission.states.npc_conversations.sessions.lock().ok()?;
        match sessions
            .get(&command.game_session_id)?
            .liquid_black
            .as_ref()
        {
            Some(State::Commit { .. }) => {
                return Some(Step {
                    context,
                    choice: None,
                });
            }
            state => state.and_then(|state| state.topic(&context, intent.npc_actor)),
        }
    };
    let values = admission
        .states
        .quest_sessions
        .lock()
        .ok()
        .and_then(|sessions| {
            let tracks = sessions
                .get(&command.game_session_id)?
                .copy
                .as_ref()?
                .tracks();
            Some((
                tracks.get(grave::QUESTLINE).copied().unwrap_or(0),
                tracks.get(grave::OUTFIT).copied().unwrap_or(-1),
                tracks.get(quest::QUESTLINE).copied().unwrap_or(-1),
                tracks.get(quest::VISITOR).copied().unwrap_or(0),
            ))
        });
    let Some((grave_questline, grave_outfit, liquid_questline, visitor)) = values else {
        return (context.2 == grave::NPC).then(|| {
            refuse_grave_unavailable(
                admission, actor, command, intent, runtime, ordinary, context,
            )
        });
    };
    let choice = {
        if context.2 == grave::NPC {
            let questline = grave_questline;
            // The ordinary owner identifies a real initial greeting. A repeated
            // hi while already engaged has no line and cannot reset the topic.
            if is_grave_greeting(admission, command, intent, ordinary) {
                Choice {
                    reply: Some(Reply::GraveGreeting {
                        started: questline >= 1,
                    }),
                    transition: None,
                    topic: None,
                    deferred: false,
                }
            } else {
                let topic = match topic {
                    Some(Topic::Grave(topic)) => Some(topic),
                    _ => None,
                };
                let choice = grave::choose(&intent.text, topic, questline, grave_outfit)?;
                Choice {
                    reply: choice.reply.map(Reply::Grave),
                    transition: choice.transition,
                    topic: choice.topic.map(Topic::Grave),
                    deferred: choice.deferred,
                }
            }
        } else {
            let topic = match topic {
                Some(Topic::Liquid(topic)) => Some(topic),
                _ => None,
            };
            let choice = quest::choose(
                quest::Npc::from_key(&context.2)?,
                &intent.text,
                topic,
                liquid_questline,
                visitor,
            )?;
            Choice {
                reply: choice.reply.map(Reply::Liquid),
                transition: choice.transition,
                topic: choice.topic.map(Topic::Liquid),
                deferred: false,
            }
        }
    };
    if let Some(reply) = choice.reply {
        reply.lines()?;
    }
    Some(Step {
        context,
        choice: Some(choice),
    })
}

fn unknown() -> NpcTalkOutcome {
    NpcTalkOutcome {
        unknown: true,
        ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
    }
}

fn source_reply(intent: &NpcTalkIntent, reply: Option<Reply>) -> NpcTalkOutcome {
    let lines = match reply {
        Some(key) => match key.lines() {
            Some(lines) => lines.to_vec(),
            None => return unknown(),
        },
        None => Vec::new(),
    };
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

fn complete(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    request: &QuestTransitionRequest,
    committed: bool,
) {
    if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
        && let Some(state) = sessions.get_mut(&session)
        && matches!(&state.liquid_black, Some(State::Commit { request: pending, .. }) if pending == request)
    {
        state.liquid_black = match state.liquid_black.take() {
            Some(State::Commit {
                context,
                intent,
                next_topic: Some(topic),
                ..
            }) if committed => Some(State::Topic {
                context,
                npc: intent.npc_actor,
                topic,
            }),
            _ => None,
        };
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
    let Some(choice) = step.choice else {
        return unknown();
    };
    if choice.deferred {
        return rejected();
    }
    let Some(transition) = choice.transition else {
        let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock() else {
            return unknown();
        };
        let state = sessions.entry(command.game_session_id).or_default();
        if matches!(state.liquid_black, Some(State::Commit { .. })) {
            return unknown();
        }
        state.liquid_black = choice.topic.map(|topic| State::Topic {
            context: step.context,
            npc: intent.npc_actor,
            topic,
        });
        drop(sessions);
        let mut outcome = source_reply(&intent, choice.reply);
        if matches!(choice.reply, Some(Reply::GraveGreeting { .. })) {
            let Some(name) = admission.states.chat.speaker_name(command.game_session_id) else {
                return unknown();
            };
            if let Some(conversation) = outcome.conversation.as_mut() {
                for line in &mut conversation.lines {
                    *line = line.replace("|PLAYERNAME|", &name);
                }
            }
        }
        return outcome;
    };
    let (Some(item_fence), Ok(id), Some(catalogue)) = (
        command.item_fence,
        CommandId::new(command.command_id),
        admission.quest_catalogue.as_ref(),
    ) else {
        return rejected();
    };
    // Q18's source start is deliberately not in the active catalogue until its
    // atomic13-effect writer is approved. Never emit success or a partial reset.
    if catalogue.transition(transition).is_none() {
        return rejected();
    }
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
        let Ok(runtime) = admission.runtime.try_lock() else {
            return rejected();
        };
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
    }
    let request = QuestTransitionRequest {
        transition_key: transition.into(),
        cause: QuestCause::Command(CommandRef::new(command.game_session_id, id)),
    };
    {
        let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock() else {
            return unknown();
        };
        let state = sessions.entry(command.game_session_id).or_default();
        if matches!(state.liquid_black, Some(State::Commit { .. })) {
            return unknown();
        }
        state.liquid_black = Some(State::Commit {
            character: fence.character_id,
            context: step.context,
            request: request.clone(),
            intent: intent.clone(),
            reply: choice.reply,
            next_topic: choice.topic,
        });
    }
    let outcome = slot
        .commit_quest_transition(
            admission.root,
            admission.character,
            admission.holder,
            fence,
            request.clone(),
            std::sync::Arc::clone(catalogue),
        )
        .await;
    drop(slot);
    match outcome {
        Ok(QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)) => {
            complete(admission, command.game_session_id, &request, true);
            admission
                .refresh_quest_session(command.game_session_id, false)
                .await;
            source_reply(&intent, choice.reply)
        }
        Err(crate::durability::character_progression::CharacterProgressionError::Unavailable(
            _,
        )) => unknown(),
        _ => {
            complete(admission, command.game_session_id, &request, false);
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
        .liquid_black
        .clone()?;
    let State::Commit {
        character,
        request,
        intent,
        reply,
        ..
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
        .reconcile_character_quest_transition(admission.character, character, request.clone())
        .await
    {
        Ok(receipt) => {
            complete(
                admission,
                command.game_session_id,
                &request,
                receipt.is_some(),
            );
            if receipt.is_some() {
                admission
                    .refresh_quest_session(command.game_session_id, false)
                    .await;
                Some(source_reply(&intent, reply))
            } else {
                Some(NpcTalkOutcome::only(NpcIntentDisposition::Rejected))
            }
        }
        Err(_) => Some(unknown()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_topic_requires_the_same_npc_and_conversation_generation() {
        let context = (1, 2, "oteryn:npc.spectulus".into());
        let npc = EntityRef {
            identity: [3; 16],
            generation: 4,
        };
        let state = State::Topic {
            context: context.clone(),
            npc,
            topic: Topic::Liquid(quest::Topic::Spectulus23),
        };
        assert_eq!(
            state.topic(&context, npc),
            Some(Topic::Liquid(quest::Topic::Spectulus23))
        );
        assert_eq!(state.topic(&(1, 3, context.2.clone()), npc), None);
        assert_eq!(state.topic(&(2, 2, context.2.clone()), npc), None);
        assert_eq!(
            state.topic(
                &context,
                EntityRef {
                    generation: 5,
                    ..npc
                }
            ),
            None
        );
        assert_eq!(
            state.topic(
                &context,
                EntityRef {
                    identity: [4; 16],
                    ..npc
                }
            ),
            None
        );
    }

    #[test]
    fn native_reply_retains_all_six_source_lines_without_item_changes() {
        let intent = NpcTalkIntent {
            npc_actor: EntityRef {
                identity: [3; 16],
                generation: 4,
            },
            text: "yes".into(),
        };
        let outcome = source_reply(&intent, Some(Reply::Liquid(quest::Reply::SpectulusYes21)));
        assert_eq!(outcome.disposition, NpcIntentDisposition::Ok);
        assert!(!outcome.unknown);
        assert!(!outcome.inventory_changed);
        assert_eq!(
            outcome.conversation.as_ref().map(|reply| reply.lines.len()),
            Some(6)
        );
        assert_eq!(
            outcome.conversation.and_then(|reply| reply.open),
            Some((intent.npc_actor, NpcConversationState::Open))
        );
    }
}
