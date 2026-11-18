//! Armenius's pre-encounter conversation from Crystal 201189f8.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;

const NPC: &str = "oteryn:npc.armenius";
const MISSION: &str = "oteryn:quest-progress/quest/u8_4/blood_brothers/mission03";

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
            let mission = tracks.get(MISSION).copied();
            let speaks_spell = intent.text.eq_ignore_ascii_case("alori mort");
            if context.2 != NPC
                || intent.text == "cookie"
                || intent.text == "yes"
                || !(mission == Some(2) || (mission == Some(1) && speaks_spell))
            {
                return None;
            }
            let (next, reply) = match topic.number {
                0 => (2, "Speak plainly or leave me alone."),
                2 => (3, "Those words unsettle me. You should leave."),
                3 => (4, "What do you think those words mean?"),
                4 => (5, "Go on, say it again. I could use another laugh."),
                _ => return None,
            };
            Some(quest_dialogue::Choice {
                transition: None,
                next_topic: Some(next.into()),
                reply: vec![reply.into()],
            })
        },
    )
}
