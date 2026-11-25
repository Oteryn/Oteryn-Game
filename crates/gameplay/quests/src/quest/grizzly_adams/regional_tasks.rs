//! Crystal 201189f, killing_in_the_name_of.lua:72–212 and
//! creaturescripts_monster_kill.lua:59–77. These hunts share the Quest
//! journal and the native party death owner with the Paw and Fur hunts.

use crate::quest::log::{JournalPiece, JournalText, QuestJournal, QuestLogMission};
use crate::quest::native_kills::Counter;
use std::collections::BTreeMap;

const QUEST: &str = super::QUEST;

struct Task {
    source_track: &'static str,
    phase: &'static str,
    count: &'static str,
    quota: i64,
    report_to: &'static str,
}

macro_rules! tasks {
    ($(($key:literal, $source:literal, $quota:literal, $npc:literal, [$($creature:literal),+]),)+) => {
        const TASKS: &[Task] = &[
            $(Task {
                source_track: concat!("canary:quest-progress/quest/u8_5/killing_in_the_name_of/", $source),
                phase: concat!("oteryn:quest-progress/hunting-tasks/", $key, "/phase"),
                count: concat!("oteryn:quest-progress/hunting-tasks/", $key, "/count"),
                quota: $quota,
                report_to: $npc,
            },)+
        ];

        pub(crate) const COUNTERS: &[Counter] = &[
            $($(Counter {
                transition: concat!("oteryn:quest-transition/native-kills/hunting-tasks/", $key, "/", $creature),
                creature: concat!("oteryn:creature.", $creature),
                quest: QUEST,
                track: concat!("oteryn:quest-progress/hunting-tasks/", $key, "/count"),
                gate: concat!("oteryn:quest-progress/hunting-tasks/", $key, "/phase"),
                maximum: $quota,
                authored_stage: false,
                world_change: None,
                death_area: None,
            },)+)+
        ];
    };
}

tasks!(
    (
        "blue-djinns",
        "blue_djinn_task",
        500,
        "Malor",
        ["blue_djinn", "marid"]
    ),
    (
        "cyclops",
        "cyclops_task",
        500,
        "Daniel Steelsoul",
        ["cyclops", "cyclops_drone", "cyclops_smith"]
    ),
    (
        "goblins",
        "goblin_task",
        150,
        "Daniel Steelsoul",
        ["goblin", "goblin_scavenger", "goblin_assassin"]
    ),
    (
        "green-djinns",
        "green_djinn_task",
        500,
        "Gabel",
        ["green_djinn", "efreet"]
    ),
    (
        "minotaurs",
        "budrik_minos",
        5000,
        "Budrik",
        [
            "minotaur",
            "minotaur_guard",
            "minotaur_mage",
            "minotaur_archer"
        ]
    ),
    (
        "necromancers",
        "lugri_necromancers",
        4000,
        "Lugri",
        [
            "necromancer",
            "priestess",
            "blood_priest",
            "blood_hand",
            "shadow_pupil"
        ]
    ),
    (
        "pirates",
        "pirate_task",
        3000,
        "Raymond Striker",
        [
            "pirate_marauder",
            "pirate_cutthroat",
            "pirate_buccaneer",
            "pirate_corsair"
        ]
    ),
    (
        "rotworms",
        "rotworm_task",
        300,
        "Daniel Steelsoul",
        ["rotworm", "carrion_worm"]
    ),
    (
        "trolls",
        "troll_task",
        100,
        "Daniel Steelsoul",
        ["troll", "troll_champion"]
    ),
);

// Unlike the pirate bounty, these reports exist to grant a one-shot boss
// entry. Keep the earned quota reportable when its physical route is admitted.
pub(crate) fn awaits_boss_route(key: &str) -> bool {
    matches!(
        key,
        "oteryn:quest-reward/hunting-tasks/blue-djinns/report"
            | "oteryn:quest-reward/hunting-tasks/green-djinns/report"
            | "oteryn:quest-reward/hunting-tasks/minotaurs/report"
            | "oteryn:quest-reward/hunting-tasks/necromancers/report"
    )
}

pub(super) fn journal(name: &str, source_track: &str) -> Option<QuestLogMission> {
    let task = TASKS
        .iter()
        .find(|task| task.source_track == source_track)?;
    Some(QuestLogMission {
        name: name.into(),
        track: task.phase.into(),
        start_value: 1,
        end_value: 2,
        journal: QuestJournal::PerStage(BTreeMap::from([
            (
                1,
                JournalText::Template(vec![
                    JournalPiece::Text("Hunted ".into()),
                    JournalPiece::Track(task.count.into()),
                    JournalPiece::Text(format!(
                        "/{}. Report to {} after the hunt.",
                        task.quota, task.report_to
                    )),
                ]),
            ),
            (
                2,
                JournalText::Text(format!("You reported this hunt to {}.", task.report_to)),
            ),
        ])),
    })
}
