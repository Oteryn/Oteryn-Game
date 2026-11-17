//! Crystal's finite text adventure. The NPC adapter owns the ephemeral topic;
//! final completion and its achievement commit together through the Quest writer.
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
    use super::{QuestComparison, QuestEffect, QuestEffectKind, QuestTransition};
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    transitions.push(QuestTransition {
        world_change: None,
        key: COMPLETE.into(),
        quest: QUEST.into(),
        effects: [
            (QUESTLINE, QuestComparison::Eq(27), 28),
            (MISSION, QuestComparison::Any, 2),
            (POTION, QuestComparison::Eq(1), 1),
        ]
        .into_iter()
        .map(|(track, from, value)| QuestEffect {
            track: track.into(),
            from,
            effect: QuestEffectKind::Set(value),
        })
        .collect(),
        // The dream ends Mission09, not the whole Wrath storyline.
        completes: false,
        experience: None,
    });
    QuestStateCatalogue::new(
        catalogue.content_revision,
        catalogue.tracks.into_values().collect(),
        transitions,
    )
    .map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nx6_finale_has_reply_and_atomic_transition() -> Result<(), String> {
        let reply = respond(27, 1, 32, "use mirror").ok_or("final reply")?;
        assert_eq!(reply.topic, 0);
        assert_eq!(reply.messages.len(), 7);
        let data =
            super::super::loader::load_embedded_quest_state("nx6").map_err(|e| format!("{e:?}"))?;
        let transition = data
            .catalogue()
            .transition(COMPLETE)
            .ok_or("final transition")?;
        assert_eq!(transition.effects.len(), 3);
        assert!(!transition.completes, "Wrath continues after the dream");
        Ok(())
    }

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
    fn sleeping_dragon_final_requires_locked_source_prerequisites() -> Result<(), String> {
        use super::super::{QuestRefusal, loader};
        let data =
            loader::load_embedded_quest_state("wrath-final-test").map_err(|e| format!("{e:?}"))?;
        let catalogue = data.catalogue();
        let finale = catalogue.transition(COMPLETE).ok_or("finale")?;
        let mut tracks = std::collections::BTreeMap::from([
            (QUESTLINE.into(), 27),
            (MISSION.into(), 1),
            (POTION.into(), 1),
        ]);
        let changes = catalogue
            .evaluate(finale, &tracks, 0)
            .map_err(|e| format!("{e:?}"))?;
        assert!(
            changes
                .iter()
                .any(|c| c.track == QUESTLINE && c.after == 28)
        );
        assert!(changes.iter().any(|c| c.track == MISSION && c.after == 2));
        tracks.insert(POTION.into(), 0);
        assert_eq!(
            catalogue.evaluate(finale, &tracks, 0),
            Err(QuestRefusal::StageMismatch)
        );
        tracks.insert(POTION.into(), 1);
        tracks.insert(QUESTLINE.into(), 28);
        assert_eq!(
            catalogue.evaluate(finale, &tracks, 0),
            Err(QuestRefusal::StageMismatch)
        );
        Ok(())
    }

    #[test]
    fn all_source_steps_end_in_the_atomic_finale() -> Result<(), String> {
        let data = dialogue().ok_or("dialogue")?;
        let mut topic = respond(27, 1, 0, &data.entry_keyword).ok_or("entry")?.topic;
        for step in data.steps.iter() {
            let reply = respond(27, 1, topic, &step.keyword).ok_or("step")?;
            assert_eq!(reply.messages, step.messages);
            topic = reply.topic;
        }
        assert_eq!(topic, 0);
        assert_eq!(
            data.steps.last().map(|step| step.keyword.as_str()),
            Some("use mirror")
        );
        assert!(respond(27, 1, topic, "use mirror").is_none());
        assert!(respond(27, 1, topic, "help").is_none());
        assert!(respond(28, 1, 32, "use mirror").is_none());
        Ok(())
    }
}
