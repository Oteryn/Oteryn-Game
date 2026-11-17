//! Native Planestrider's exact final dialogue reaches the normal replayable QuestState writer.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use oteryn_gameplay_quests::quest::planegazer_finale as quest;
pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> Option<quest_dialogue::Step> {
    quest_dialogue::prepare(
        admission,
        actor,
        command,
        intent,
        runtime,
        |context, topic, _, tracks| {
            if context.2 != quest::NPC || tracks.get(quest::READY).copied().unwrap_or(0) != 1 {
                return None;
            }
            let (next, complete) = quest::respond(topic.number, &intent.text)?;
            Some(quest_dialogue::Choice {
                transition: complete.then(|| quest::COMPLETE.into()),
                next_topic: Some(next.into()),
                // UI-copy difference: retain exact source control words and grant event, while
                // unavailable story paragraphs are suppressed rather than invented.
                reply: if complete {
                    vec!["You have earned the title Planegazer.".into()]
                } else {
                    Vec::new()
                },
            })
        },
    )
}
pub(super) async fn apply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcTalkIntent,
    step: quest_dialogue::Step,
) -> NpcTalkOutcome {
    quest_dialogue::apply(admission, actor, command, intent, step).await
}
