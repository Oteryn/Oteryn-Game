//! Crystal's finite text adventure. The NPC adapter owns the ephemeral topic;
//! final completion and its achievement remain deferred together.
//! zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:
//! data-global/npc/a_sleeping_dragon.lua:67-198.

use super::QuestStateCatalogue;
use serde::Deserialize;

pub const NPC: &str = "oteryn:npc.a_sleeping_dragon";
pub const QUEST: &str = "oteryn:quest.wrath_of_the_emperor_quest";
pub const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/questline";
pub const MISSION: &str = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/mission09";
pub const POTION: &str =
    "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/interdimensional_potion";
pub const COMPLETE: &str = "oteryn:quest-transition/wrath-of-the-emperor/dragon-dream-complete";
pub const ACHIEVEMENT: &str = "Wayfarer";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Dialogue {
    schema: u8,
    source: String,
    source_blob_sha256: String,
    npc: String,
    entry_keyword: String,
    entry_messages: Vec<String>,
    help_messages: Vec<String>,
    steps: Vec<Step>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Step {
    topic: u8,
    keyword: String,
    next_topic: u8,
    messages: Vec<String>,
    source_line: u16,
}

fn dialogue() -> Option<&'static Dialogue> {
    static DATA: std::sync::LazyLock<Option<Dialogue>> = std::sync::LazyLock::new(|| {
        let data: Dialogue = serde_json::from_str(include_str!(
            "../../../../../content/dialogues/quests-2/wrath-sleeping-dragon.json"
        ))
        .ok()?;
        let messages_fit = |messages: &[String]| {
            !messages.is_empty()
                && messages.len() <= 32
                && messages
                    .iter()
                    .all(|line| !line.is_empty() && line.len() <= 2048)
        };
        if data.schema != 1
            || data.source
                != "zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/a_sleeping_dragon.lua:67-198"
            || data.source_blob_sha256
                != "58d58e2b0f81be49c29082555c6486175d81817a0f62d1c56f30bc0fc510385a"
            || data.npc != NPC
            || data.entry_keyword != "SOLOSARASATIQUARIUM"
            || data.steps.len() != 32
            || !messages_fit(&data.entry_messages)
            || !messages_fit(&data.help_messages)
            || data.steps.iter().enumerate().any(|(index, step)| {
                usize::from(step.topic) != index + 1
                    || usize::from(step.next_topic) != if index == 31 { 0 } else { index + 2 }
                    || !(86..=185).contains(&step.source_line)
                    || step.keyword.is_empty()
                    || !messages_fit(&step.messages)
            })
        {
            return None;
        }
        Some(data)
    });
    DATA.as_ref()
}

#[derive(Debug, PartialEq, Eq)]
pub struct Reply {
    pub topic: u8,
    pub messages: &'static [String],
}

/// The opening word is case-sensitive. Every subsequent source branch compares
/// the whole lowercased message; neither whitespace trimming nor keyword contains
/// is added. A wrong input retains the current topic and makes no durable write.
pub fn respond(questline: i64, potion: i64, topic: u8, message: &str) -> Option<Reply> {
    if questline != 27 {
        return None;
    }
    let data = dialogue()?;
    if message == data.entry_keyword && potion == 1 {
        return Some(Reply {
            topic: 1,
            messages: &data.entry_messages,
        });
    }
    let lowered = message.to_ascii_lowercase();
    if lowered == "help" && (1..34).contains(&topic) {
        return Some(Reply {
            topic,
            messages: &data.help_messages,
        });
    }
    let step = data
        .steps
        .iter()
        .find(|step| step.topic == topic && step.keyword == lowered)?;
    if topic == 32 {
        // Crystal a_sleeping_dragon.lua:185-198 awards Wayfarer together with
        // QuestLine=28 and Mission09=2. Its shared writer hook is out of scope:
        // https://github.com/Oteryn/Oteryn-Game/pull/2844#issuecomment-6589902675
        // Keep the source reply in the capture, without claiming completion.
        return None;
    }
    Some(Reply {
        topic: step.next_topic,
        messages: &step.messages,
    })
}

pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    if dialogue().is_none() {
        return Err("Sleeping Dragon dialogue source or bounds".into());
    }
    for key in [QUESTLINE, MISSION, POTION] {
        if catalogue
            .track(key)
            .is_none_or(|track| track.quest != QUEST)
        {
            return Err(format!("Sleeping Dragon source track {key}"));
        }
    }
    Ok(catalogue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_requires_source_case_stage_and_used_potion() {
        assert!(respond(27, -1, 0, "SOLOSARASATIQUARIUM").is_none());
        assert!(respond(26, 1, 0, "SOLOSARASATIQUARIUM").is_none());
        assert!(respond(28, 1, 0, "SOLOSARASATIQUARIUM").is_none());
        assert!(respond(27, 1, 0, "solosarasatiquarium").is_none());
        assert!(respond(27, 1, 0, " SOLOSARASATIQUARIUM").is_none());
        assert_eq!(
            respond(27, 1, 0, "SOLOSARASATIQUARIUM").map(|r| r.topic),
            Some(1)
        );
    }

    #[test]
    fn wrong_words_and_help_do_not_skip_a_source_step() -> Result<(), String> {
        assert!(respond(27, 1, 1, "take attachment").is_none());
        assert!(respond(27, 1, 1, "west ").is_none());
        let help = respond(27, 1, 2, "HELP").ok_or("help")?;
        assert_eq!(help.topic, 2);
        assert_eq!(
            respond(27, 1, 2, "TAKE ATTACHMENT")
                .ok_or("attachment")?
                .topic,
            3
        );
        assert!(respond(27, 1, 0, "help").is_none());
        Ok(())
    }

    #[test]
    fn sleeping_dragon_final_is_deferred_with_no_success_or_transition() -> Result<(), String> {
        // ANSWER6589902675: the source completion includes Wayfarer and needs
        // a new shared writer hook. All completion effects stay Left out together.
        assert!(
            respond(27, 1, 32, "use mirror").is_none(),
            "no success text or state-only completion without the achievement owner"
        );
        let data = super::super::loader::parse_quest_state(
            include_str!("../../../../../content/quests/missions/quest-state.json"),
            "wrath-final-test",
        )
        .map_err(|e| format!("{e:?}"))?;
        let catalogue = extend(data.catalogue().clone())?;
        assert!(catalogue.transition(COMPLETE).is_none());
        Ok(())
    }

    #[test]
    fn first_31_source_steps_stop_before_the_deferred_atomic_finale() -> Result<(), String> {
        let data = dialogue().ok_or("dialogue")?;
        let mut topic = respond(27, 1, 0, &data.entry_keyword).ok_or("entry")?.topic;
        for step in data.steps.iter().take(31) {
            let reply = respond(27, 1, topic, &step.keyword).ok_or("step")?;
            assert_eq!(reply.messages, step.messages);
            topic = reply.topic;
        }
        assert_eq!(topic, 32);
        assert_eq!(
            data.steps.last().map(|step| step.keyword.as_str()),
            Some("use mirror")
        );
        assert!(respond(27, 1, topic, "use mirror").is_none());
        assert_eq!(respond(27, 1, topic, "help").ok_or("final help")?.topic, 32);
        assert!(respond(28, 1, 32, "use mirror").is_none());
        Ok(())
    }
}
