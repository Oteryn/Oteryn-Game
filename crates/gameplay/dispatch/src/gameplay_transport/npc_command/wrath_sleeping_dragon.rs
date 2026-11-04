//! Live NPC caller for Crystal's Sleeping Dragon dream conversation.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use crate::foundation::CommandId;
use oteryn_gameplay_quests::quest::wrath_sleeping_dragon as quest;

#[derive(Default)]
pub(super) struct Conversation {
    context: Option<(u64, u64, String)>,
    topic: u8,
}

pub(super) enum Step {
    Refused,
    Reply {
        context: (u64, u64, String),
        previous_topic: u8,
        reply: quest::Reply,
    },
}

fn reply(intent: &NpcTalkIntent, messages: Vec<String>) -> NpcTalkOutcome {
    NpcTalkOutcome {
        unknown: false,
        inventory_changed: false,
        disposition: NpcIntentDisposition::Ok,
        conversation: Some(NpcConversation {
            open: Some((intent.npc_actor, NpcConversationState::Open)),
            lines: messages,
        }),
        window: None,
    }
}

fn is_source_greeting(intent: &NpcTalkIntent, ordinary: &NpcTalkOutcome) -> bool {
    ordinary.disposition == NpcIntentDisposition::Ok
        && oteryn_gameplay_npc::quest_greetings::get(quest::NPC).is_some_and(|greeting| {
            ordinary.conversation.as_ref().is_some_and(|conversation| {
                conversation.open == Some((intent.npc_actor, NpcConversationState::Open))
                    && conversation.lines.len() == 1
                    && conversation.lines[0] == greeting.greet.text
            })
        })
}

fn refuse_unavailable(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
    ordinary: &NpcTalkOutcome,
) -> Step {
    // Crystal a_sleeping_dragon.lua:49-55 selects the greeting from Questline.
    // A failed copy read cannot select the default. The caller still holds the
    // runtime lock; no quest/session mutex is held here. Undo only the exact NPC's
    // freshly opened greeting, so a later retry can actually greet again.
    if is_source_greeting(intent, ordinary)
        && let Some(npc) =
            service_context::live_npc(runtime, actor, command.game_session_id, intent.npc_actor)
    {
        admission
            .states
            .chat
            .end_engagement(command.game_session_id, npc.actor);
    }
    // No request was prepared or committed: this is not an unknown receipt.
    Step::Refused
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
    if context.2 != quest::NPC {
        return None;
    }
    let previous_topic = admission
        .states
        .npc_conversations
        .sessions
        .lock()
        .ok()
        .map(|sessions| {
            sessions
                .get(&command.game_session_id)
                .filter(|state| state.dragon.context.as_ref() == Some(&context))
                .map_or(0, |state| state.dragon.topic)
        });
    let Some(previous_topic) = previous_topic else {
        return Some(refuse_unavailable(
            admission, actor, command, intent, runtime, ordinary,
        ));
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
                tracks.get(quest::QUESTLINE).copied().unwrap_or(-1),
                tracks.get(quest::POTION).copied().unwrap_or(-1),
            ))
        });
    let Some((questline, potion)) = values else {
        return Some(refuse_unavailable(
            admission, actor, command, intent, runtime, ordinary,
        ));
    };
    if questline == 27 && is_source_greeting(intent, ordinary) {
        // Crystal a_sleeping_dragon.lua:49-55 changes only an actual greeting,
        // not every unrecognised word spoken while the quest is at stage27.
        static DREAM_GREETING: std::sync::LazyLock<Option<Vec<String>>> =
            std::sync::LazyLock::new(|| {
                Some(vec![
                    oteryn_gameplay_npc::quest_greetings::get(quest::NPC)?
                        .variants
                        .get("dream")?
                        .text
                        .clone(),
                ])
            });
        return Some(Step::Reply {
            context,
            previous_topic,
            reply: quest::Reply {
                topic: 0,
                messages: DREAM_GREETING.as_ref()?.as_slice(),
            },
        });
    }
    let response = quest::respond(questline, potion, previous_topic, &intent.text)?;
    Some(Step::Reply {
        context,
        previous_topic,
        reply: response,
    })
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
    let Step::Reply {
        context,
        previous_topic,
        reply: response,
    } = step
    else {
        return rejected();
    };
    let (Some(item_fence), Ok(_)) = (command.item_fence, CommandId::new(command.command_id)) else {
        return rejected();
    };
    let _slot = admission
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
            ) != Some(context.clone())
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
    {
        let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock() else {
            return rejected();
        };
        let state = &mut sessions.entry(command.game_session_id).or_default().dragon;
        let current_topic = if state.context.as_ref() == Some(&context) {
            state.topic
        } else {
            0
        };
        if current_topic != previous_topic {
            return rejected();
        }
        state.context = Some(context);
        state.topic = response.topic;
    }
    reply(&intent, response.messages.to_vec())
}
