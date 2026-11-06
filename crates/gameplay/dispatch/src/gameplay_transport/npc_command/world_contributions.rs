//! Shared donations use the existing conversation and fenced Quest transaction.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::quest_dialogue::{self, Choice, Step, Topic};
use super::*;
use oteryn_gameplay_quests::quest::world_contributions as quest;

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> Option<Step> {
    quest_dialogue::prepare(
        admission,
        actor,
        command,
        intent,
        runtime,
        |context, topic, _, _| {
            // The common conversation owner also hosts other NPC adapters. A
            // private tag prevents their numeric topics becoming a donation.
            let number = if topic.value.as_deref() == Some("world-contribution") {
                topic.number
            } else {
                0
            };
            let choice = quest::dialogue(&context.2, &intent.text, number)?;
            Some(Choice {
                transition: choice.transition.map(str::to_owned),
                next_topic: Some(Topic {
                    number: choice.topic,
                    value: Some("world-contribution".into()),
                }),
                reply: vec![choice.reply.into()],
            })
        },
    )
}
