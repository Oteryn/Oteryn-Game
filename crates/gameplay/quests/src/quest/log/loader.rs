//! Compose QUEST-CONTENT-2 journals against the existing admitted state catalogue.
//! Unsupported source bindings are recorded explicitly, never approximated.

use serde::Deserialize;

use super::super::loader::LoweredQuestState;
use super::journal_source::{Journal, track};
use super::source_texts::SourceTexts;
use super::{
    QuestJournal, QuestLogCatalogue, QuestLogContent, QuestLogKind, QuestLogMission, QuestLogQuest,
};

const APE_CITY: &str = "oteryn:quest.the_ape_city_quest";
const APE_CITY_STAGE: &str = "oteryn:quest-progress/chosen/ape-city/stage";

const QUESTS: &str = include_str!(
    "../../../../../../tools/content-schema/quest-authoring/samples/questlog/quests.json"
);
const TEXTS: &str = include_str!(
    "../../../../../../tools/content-schema/quest-authoring/samples/source_texts/source_texts.json"
);

/// CP 2026-10-22: content without an admitted track/ordered binding is excluded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestLogExclusion {
    pub quest: String,
    pub mission: Option<String>,
    pub reason: &'static str,
}

/// Runtime content plus the fact-only boot audit of excluded source entries.
pub struct LoadedQuestLog {
    pub content: QuestLogContent,
    pub exclusions: Vec<QuestLogExclusion>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    quests: Vec<Quest>,
}

#[derive(Deserialize)]
struct Quest {
    identity: Identity,
    display_name: String,
    kind: String,
    shown_in_quest_log: bool,
    start: Option<Start>,
    #[serde(default)]
    hide_when_completed: bool,
    #[serde(default)]
    missions: Vec<Mission>,
}

#[derive(Deserialize)]
struct Identity {
    key: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Start {
    progress: String,
    at_least: i64,
}

#[derive(Deserialize)]
struct Mission {
    key: String,
    name: String,
    progress: String,
    start_value: i64,
    end_value: i64,
    journal: Journal,
}

/// Load the merged journal sources without adding tracks, writers or content pins.
pub fn load_embedded_quest_log(lowered: &LoweredQuestState) -> Result<LoadedQuestLog, String> {
    parse(QUESTS, TEXTS, lowered)
}

fn parse(quests: &str, texts: &str, lowered: &LoweredQuestState) -> Result<LoadedQuestLog, String> {
    let document: Document = serde_json::from_str(quests).map_err(|e| e.to_string())?;
    let texts = SourceTexts::parse(texts)?;
    let mut entries = Vec::new();
    let mut exclusions = Vec::new();
    for quest in document.quests {
        let source = quest
            .identity
            .key
            .split_once(":quest/")
            .map(|(_, key)| key)
            .ok_or("foreign journal quest")?;
        let key = format!("oteryn:quest.{source}");
        let kind = match quest.kind.as_str() {
            "storyline" => QuestLogKind::Storyline,
            "reward_only" => QuestLogKind::RewardOnly,
            "script_only" => QuestLogKind::ScriptOnly,
            _ => return Err("unknown journal quest kind".into()),
        };
        let ape_city_stage = key == APE_CITY
            && lowered
                .catalogue()
                .track(APE_CITY_STAGE)
                .is_some_and(|track| track.quest == key && track.min == 0 && track.max == 15);
        let start = if ape_city_stage {
            Some((APE_CITY_STAGE.into(), 1))
        } else if key == super::super::grizzly_adams::QUEST {
            // The generic source export left this quest's start unbound. Crystal
            // grizzly_adams.lua:412-416 explicitly opens QuestLogEntry at zero on join.
            Some((
                super::super::grizzly_adams::storage_track(
                    super::super::grizzly_adams::QUEST_LOG_STORAGE,
                ),
                0,
            ))
        } else {
            quest
                .start
                .as_ref()
                .and_then(|start| track(lowered, &start.progress).map(|key| (key, start.at_least)))
        };
        if lowered.catalogue().definition_hash(&key).is_none()
            || (quest.start.is_some() && start.is_none())
        {
            exclusions.push(QuestLogExclusion {
                quest: key,
                mission: None,
                reason: "no admitted progress track",
            });
            continue;
        }
        let mut missions = Vec::new();
        for mission in quest.missions {
            if ape_city_stage && mission.key == "hairycles_missions" {
                missions.push(ape_city_mission(mission.name));
                continue;
            }
            if key == super::super::grizzly_adams::QUEST
                && let Some(native) = super::super::grizzly_adams::journal(
                    &mission.name,
                    &mission.progress,
                    mission.start_value,
                    mission.end_value,
                )
            {
                missions.push(native);
                continue;
            }
            let binding = track(lowered, &mission.progress);
            let journal = mission.journal.lower(lowered, &texts)?;
            let (Some(track), Some(mut journal)) = (binding, journal) else {
                exclusions.push(QuestLogExclusion {
                    quest: key.clone(),
                    mission: Some(mission.key),
                    reason: "no admitted progress track",
                });
                continue;
            };
            let mut start_value = mission.start_value;
            if track == "oteryn:quest-progress/quest/u8_1/tibia_tales/aritos_task" {
                // The sealed cave blocks the peace negotiation. Keep the later
                // journal stages for characters who already reached them.
                start_value = 2;
                if let QuestJournal::PerStage(stages) = &mut journal {
                    stages.remove(&1);
                }
            }
            missions.push(QuestLogMission {
                name: mission.name,
                track,
                start_value,
                end_value: mission.end_value,
                journal,
            });
        }
        entries.push(QuestLogQuest {
            title: quest.display_name,
            key,
            kind,
            shown_in_quest_log: quest.shown_in_quest_log,
            start,
            hide_when_completed: quest.hide_when_completed,
            missions,
        });
    }
    entries.retain(|entry| entry.key != super::super::monk_shrines::QUEST);
    entries.push(super::super::monk_shrines::journal());
    entries.retain(|entry| entry.key != super::super::kill_objectives::QUEST);
    entries.push(super::super::kill_objectives::journal());
    if let Some(journal) = blind_the_enemy_journal(lowered) {
        entries.retain(|entry| entry.key != journal.key);
        entries.push(journal);
    }
    super::super::world_change_journals::append_journals(&mut entries)?;
    let catalogue = QuestLogCatalogue::new(lowered.catalogue(), entries)
        .map_err(|e| format!("journal catalogue: {e:?}"))?;
    Ok(LoadedQuestLog {
        content: QuestLogContent::new(catalogue, Vec::new()),
        exclusions,
    })
}

fn blind_the_enemy_journal(lowered: &LoweredQuestState) -> Option<QuestLogQuest> {
    use super::JournalText;
    let key = "oteryn:quest.to_blind_the_enemy_quest";
    let mission = "oteryn:quest-progress/to-blind-the-enemy/mission";
    lowered.catalogue().track(mission)?;
    Some(QuestLogQuest {
        key: key.into(),
        title: "To Blind the Enemy Quest".into(),
        kind: QuestLogKind::Storyline,
        shown_in_quest_log: true,
        start: Some((mission.into(), 1)),
        hide_when_completed: false,
        missions: vec![QuestLogMission {
            name: "To Blind the Enemy".into(),
            track: mission.into(),
            start_value: 1,
            end_value: 4,
            journal: QuestJournal::PerStage(
                [
                    (
                        1,
                        "Take Eroth's mirror to the top of Elvenbane's central tower.",
                    ),
                    (2, "Use the mirror on the scrying ball."),
                    (
                        3,
                        "Recover the four tower treasures and report back to Eroth.",
                    ),
                    (4, "You blinded the enemy and received Eroth's reward."),
                ]
                .into_iter()
                .map(|(value, text)| (value, JournalText::Text(text.into())))
                .collect(),
            ),
        }],
    })
}

fn ape_city_mission(name: String) -> QuestLogMission {
    use super::{JournalText, QuestJournal};

    // The playable route uses its own stage numbers and tasks. In particular,
    // Hairycles asks for a banana and parchment where the donor journal describes
    // alternate tasks, so these entries describe the stages that actually commit.
    let entries = [
        (1, "Find a banana for Hairycles."),
        (2, "Bring the banana to Hairycles."),
        (3, "Ask Hairycles for your next mission."),
        (4, "Recover the old parchment for Hairycles."),
        (5, "Bring the parchment to Hairycles."),
        (6, "Decipher the signs for Hairycles."),
        (7, "Recover a hair from the holy ape."),
        (8, "Bring the holy ape hair to Hairycles."),
        (9, "Ask Hairycles about the catacombs."),
        (10, "Take the Snake Destroyer into Banuta."),
        (
            11,
            "Use the Snake Destroyer on the amphorae and enter the deeper catacombs.",
        ),
        (12, "Set the six levers and open the catacombs gate."),
        (13, "Destroy the snake statue beyond the gate."),
        (14, "Report back to Hairycles."),
        (
            15,
            "You completed Hairycles' missions and earned the shaman outfit.",
        ),
    ];
    QuestLogMission {
        name,
        track: APE_CITY_STAGE.into(),
        start_value: 1,
        end_value: 15,
        journal: QuestJournal::PerStage(
            entries
                .into_iter()
                .map(|(stage, text)| (stage, JournalText::Text(text.into())))
                .collect(),
        ),
    }
}

#[cfg(test)]
#[path = "loader_tests.rs"]
mod tests;
