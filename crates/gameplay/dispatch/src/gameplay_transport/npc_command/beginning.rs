//! Tutorial source greetings use the existing fenced quest conversation owner.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::quest_dialogue::{self, Choice, Step, Topic};
use super::*;
use oteryn_gameplay_quests::quest::beginning as quest;

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> Option<Step> {
    let greeting = oteryn_gameplay_npc::greeting::words(&intent.text)
        .iter()
        .any(|word| matches!(word.as_str(), "hi" | "hello" | "hail"));
    if !greeting {
        return None;
    }
    quest_dialogue::prepare(
        admission,
        actor,
        command,
        intent,
        runtime,
        |context, _, fresh, tracks| {
            if context.2 != quest::SANTIAGO || !fresh {
                return None;
            }
            let progress = tracks.get(quest::GREET).copied().unwrap_or(-1);
            if progress > 1 {
                return None;
            }
            Some(Choice {
                transition: (progress < 1).then(|| quest::START.into()),
                next_topic: Some(Topic::from(0)),
                reply: vec![
                    if progress < 1 {
                        oteryn_gameplay_npc::npc_talk::BEGINNING_GREETING
                    } else {
                        oteryn_gameplay_npc::npc_talk::BEGINNING_REPEAT_GREETING
                    }
                    .into(),
                ],
            })
        },
    )
}
