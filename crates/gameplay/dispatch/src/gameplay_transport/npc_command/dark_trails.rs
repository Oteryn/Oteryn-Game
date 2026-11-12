//! Dark Trails chooses source-bound state transitions; the common owner executes them.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::quest_dialogue::{self, Choice, Step, Topic};
use super::*;
use oteryn_gameplay_quests::quest::dark_trails as quest;

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
        |context, topic, fresh, tracks| {
            let rule = book.select(&context.2, &intent.text, topic.number, tracks)?;
            if rule.greeting_only && !fresh {
                return None;
            }
            Some(Choice {
                transition: (!rule.sets.is_empty()).then(|| quest::transition_key(&rule.key)),
                next_topic: rule.next_topic.map(Topic::from),
                reply: book.replies.get(&rule.reply)?.lines.clone(),
            })
        },
    )
}
