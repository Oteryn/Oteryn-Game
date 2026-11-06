//! Source-qualified state-only NPC calls use the existing conversation and quest writer.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use oteryn_gameplay_quests::quest::state_dialogue as quest;

pub(super) enum Step {
    Refused,
    Unfocused(Vec<String>),
    Dialogue(quest_dialogue::Step),
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
    let book = quest::load().ok()?;
    if !book.owns(&context.2) {
        return None;
    }
    let greeting = ordinary.conversation.as_ref().is_some_and(|conversation| {
        conversation.open == Some((intent.npc_actor, NpcConversationState::Open))
            && admission.states.chat.npc_talk().is_some_and(|book| {
                book.get(&context.2)
                    .is_some_and(|entry| conversation.lines == [entry.greeting()])
            })
    });
    let language = {
        let sessions = admission.states.quest_sessions.lock().ok()?;
        let copy = sessions
            .get(&command.game_session_id)
            .and_then(|entry| entry.copy.as_ref());
        copy.map(|copy| {
            (
                oteryn_gameplay_npc::quest_greetings::select(&context.2, Some(copy.tracks())),
                oteryn_gameplay_npc::quest_greetings::permits_conversation(
                    &context.2,
                    copy.tracks(),
                ),
            )
        })
    };
    if language.is_none() || (greeting && language.is_some_and(|(_, allowed)| !allowed)) {
        if greeting
            && let Some(npc) =
                service_context::live_npc(runtime, actor, command.game_session_id, intent.npc_actor)
        {
            admission
                .states
                .chat
                .end_engagement(command.game_session_id, npc.actor);
        }
        return Some(match language {
            Some((Some(line), false)) => Step::Unfocused(vec![line.into()]),
            _ => Step::Refused,
        });
    }
    if !greeting
        && ordinary
            .conversation
            .as_ref()
            .is_some_and(|conversation| !conversation.lines.is_empty())
    {
        return None;
    }
    let catalogue = admission.quest_catalogue.as_ref()?;
    quest_dialogue::prepare(
        admission,
        actor,
        command,
        intent,
        runtime,
        |_, topic, _, tracks| {
            if greeting && context.2 == "oteryn:npc.a_restless_soul" {
                return Some(quest_dialogue::Choice {
                    transition: None,
                    next_topic: Some(0.into()),
                    reply: vec![language?.0?.into()],
                });
            }
            let branch = book.respond(&context.2, &intent.text, topic.number, tracks, catalogue)?;
            Some(quest_dialogue::Choice {
                transition: branch.transition(),
                next_topic: Some(branch.next_topic.into()),
                reply: branch.reply.clone(),
            })
        },
    )
    .map(Step::Dialogue)
}

pub(super) async fn apply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcTalkIntent,
    step: Step,
) -> NpcTalkOutcome {
    match step {
        Step::Refused => NpcTalkOutcome::only(NpcIntentDisposition::Rejected),
        Step::Unfocused(lines) => NpcTalkOutcome {
            unknown: false,
            inventory_changed: false,
            disposition: NpcIntentDisposition::Ok,
            conversation: Some(NpcConversation { open: None, lines }),
            window: None,
        },
        Step::Dialogue(step) => {
            quest_dialogue::apply(admission, actor, command, intent, step).await
        }
    }
}
