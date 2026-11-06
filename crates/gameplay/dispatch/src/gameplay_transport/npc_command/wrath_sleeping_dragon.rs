//! Live Sleeping Dragon dialogue, including the recoverable atomic finale.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use oteryn_gameplay_quests::quest::wrath_sleeping_dragon as quest;

pub(super) enum Step {
    Refused,
    Dialogue(quest_dialogue::Step),
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
    let available = admission
        .states
        .quest_sessions
        .lock()
        .ok()
        .is_some_and(|sessions| {
            sessions
                .get(&command.game_session_id)
                .is_some_and(|s| s.copy.is_some())
        });
    if !available {
        return Some(refuse_unavailable(
            admission, actor, command, intent, runtime, ordinary,
        ));
    }
    quest_dialogue::prepare(
        admission,
        actor,
        command,
        intent,
        runtime,
        |_, topic, _, tracks| {
            let questline = tracks.get(quest::QUESTLINE).copied().unwrap_or(-1);
            if questline == 27 && is_source_greeting(intent, ordinary) {
                let greeting = oteryn_gameplay_npc::quest_greetings::get(quest::NPC)?;
                return Some(quest_dialogue::Choice {
                    transition: None,
                    next_topic: Some(0.into()),
                    reply: vec![greeting.variants.get("dream")?.text.clone()],
                });
            }
            let topic = u8::try_from(topic.number).ok()?;
            let response = quest::respond(
                questline,
                tracks.get(quest::POTION).copied().unwrap_or(-1),
                topic,
                &intent.text,
            )?;
            Some(quest_dialogue::Choice {
                transition: (topic == 32 && response.topic == 0).then(|| quest::COMPLETE.into()),
                next_topic: Some(u16::from(response.topic).into()),
                reply: response.messages.to_vec(),
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
        Step::Dialogue(step) => {
            quest_dialogue::apply(admission, actor, command, intent, step).await
        }
    }
}
