//! Dark Trails chooses source-bound state transitions; the common owner executes them.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::quest_dialogue::{self, Choice, Step, Topic};
use super::*;
use oteryn_gameplay_quests::quest::dark_trails as quest;

const SEANCE_COMPLETE: &str = "oteryn:quest-progress/quest/u10_50/dark_trails/mission16";

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
            if context.2 == quest::MARVIN {
                return marvin(&book, &intent.text, topic.number, tracks);
            }
            if context.2 == quest::SHOLLEY && quest::message_contains(&intent.text, "friend") {
                return sholley(&book, tracks);
            }
            if context.2 == "oteryn:npc.barnabas_dee"
                && !quest::message_contains(&intent.text, "mission")
                && quest::message_contains(&intent.text, "seance")
                && tracks.get(SEANCE_COMPLETE).copied().unwrap_or(-1) < 1
            {
                // Crystal's hand-in checks carried pollen. It records no personal harvest credit.
                return Some(Choice {
                    transition: None,
                    next_topic: Some(0.into()),
                    reply: vec!["Sorry, not possible.".into()],
                });
            }
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

fn marvin(
    book: &quest::Book,
    text: &str,
    topic: u16,
    tracks: &std::collections::BTreeMap<String, i64>,
) -> Option<Choice> {
    let contains = |word| quest::message_contains(text, word);
    let (line, next_topic, transition) = if contains("funding") {
        if tracks.get(quest::MISSION07) == Some(&1)
            && tracks.get(quest::VOTES).is_some_and(|&value| value > 0)
        {
            ("source-only:marvin:59", 1, None)
        } else {
            ("source-only:marvin:65", topic, None)
        }
    } else if contains("archives") {
        (topic == 1).then_some(("source-only:marvin:69", 2, None))?
    } else if contains("1") {
        (topic == 2).then_some(("source-only:marvin:74", 3, None))?
    } else if contains("yes") && topic == 3 {
        ("source-only:marvin:84", 0, Some(quest::ARCHIVE_VOTE.into()))
    } else {
        return None;
    };
    Some(Choice {
        transition,
        next_topic: Some(next_topic.into()),
        reply: book.replies.get(line)?.lines.clone(),
    })
}

fn sholley(book: &quest::Book, tracks: &std::collections::BTreeMap<String, i64>) -> Option<Choice> {
    (tracks.get(quest::MISSION12) == Some(&1)
        && tracks.get(quest::MISSION13).copied().unwrap_or(-1) < 1
        && tracks
            .get(quest::VOTES)
            .is_some_and(|&votes| votes >= quest::COMMONER_MIN_VOTES))
    .then_some(())?;
    Some(Choice {
        transition: Some(quest::COMMONER_CREDIT.into()),
        next_topic: Some(0.into()),
        reply: book.replies.get("source-only:sholley:59")?.lines.clone(),
    })
}
