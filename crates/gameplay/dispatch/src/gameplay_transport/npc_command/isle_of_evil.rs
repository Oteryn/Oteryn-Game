//! Source-scoped Isle dialogue selection; the shared QuestState caller owns execution.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::quest_dialogue::{self, Choice, Step, Topic};
use super::*;
use oteryn_gameplay_quests::quest::isle_of_evil as quest;

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> Option<Step> {
    let book = quest::load().ok()?;
    quest_dialogue::prepare(
        admission,
        actor,
        command,
        intent,
        runtime,
        |context, topic, _fresh, tracks| {
            let decision = book.respond(&context.2, &intent.text, topic.number, tracks)?;
            if decision.held {
                return None;
            }
            Some(Choice {
                transition: decision.transition,
                next_topic: decision.next_topic.map(Topic::from),
                reply: decision.lines,
            })
        },
    )
}
