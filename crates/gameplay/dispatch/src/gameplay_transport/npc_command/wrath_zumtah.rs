use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use oteryn_gameplay_quests::quest::wrath_zumtah as quest;

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
        |npc, topic, _, tracks| {
            if npc.2 != quest::NPC {
                return None;
            }
            let reply = quest::respond(
                tracks.get(quest::STATUS) == Some(&1),
                topic.number,
                &intent.text,
            )?;
            Some(quest_dialogue::Choice {
                transition: reply.transition.map(str::to_owned),
                next_topic: Some(reply.topic.into()),
                reply: vec![reply.line.into()],
            })
        },
    )
}
