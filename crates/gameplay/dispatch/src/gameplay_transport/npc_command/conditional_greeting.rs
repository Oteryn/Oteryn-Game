//! Read-only source voices share the normal conversation validation and reply path.
use super::quest_dialogue;
use super::*;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::UseCommand};

pub(super) struct Step {
    context: quest_dialogue::Context,
    entry: super::super::npc_talk::NpcTalkEntry,
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
    let book = admission.states.chat.npc_talk()?;
    let entry = book.get(&context.2)?;
    if entry.greeting_items().next().is_none()
        || !ordinary.conversation.as_ref().is_some_and(|conversation| {
            conversation.open == Some((intent.npc_actor, NpcConversationState::Open))
                && conversation.lines == [entry.greeting()]
        })
    {
        return None;
    }
    Some(Step {
        context,
        entry: entry.clone(),
    })
}

pub(super) async fn apply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcTalkIntent,
    step: Step,
) -> NpcTalkOutcome {
    let observed = async {
        let fence = admission
            .current_quest_fence(command.game_session_id)
            .await
            .ok()??;
        admission
            .root
            .read_quest_carried_counts(
                admission.character,
                fence,
                step.entry.greeting_items().map(str::to_owned).collect(),
            )
            .await
            .ok()
    }
    .await;
    let runtime = admission.runtime.lock().await;
    let dialogue = quest_dialogue::prepare(
        admission,
        actor,
        command,
        &intent,
        &runtime,
        |context, _, _, tracks| {
            if context != &step.context {
                return None;
            }
            Some(quest_dialogue::Choice {
                transition: None,
                next_topic: None,
                reply: vec![
                    step.entry
                        .conditional_greeting(Some(tracks), observed.as_ref())?
                        .into(),
                ],
            })
        },
    );
    if dialogue.is_none()
        && admission
            .states
            .npc_conversations
            .current_npc(
                &admission.states.chat,
                &runtime,
                actor,
                command.game_session_id,
                &intent,
            )
            .as_ref()
            == Some(&step.context)
        && let Some(npc) =
            service_context::live_npc(&runtime, actor, command.game_session_id, intent.npc_actor)
    {
        admission
            .states
            .chat
            .end_engagement(command.game_session_id, npc.actor);
    }
    drop(runtime);
    match dialogue {
        Some(dialogue) => quest_dialogue::apply(admission, actor, command, intent, dialogue).await,
        None => NpcTalkOutcome::only(NpcIntentDisposition::Rejected),
    }
}
