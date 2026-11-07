//! Authored bridge: source lacks the stage5->6 occurrence hook; legs remain separate.
use super::*;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use std::collections::BTreeMap;

const QUEST: &str = "oteryn:quest.the_beginning_quest";
const TRACK: &str = "oteryn:quest-progress/native-kills/beginning/cockroaches";
const GATE: &str =
    "oteryn:quest-progress/quest/u8_2/the_beginning_quest/santiago_npc_greet_storage";
const LOG: &str = "oteryn:quest-progress/quest/u8_2/the_beginning_quest/santiago_quest_log";
const KILL: &str = "oteryn:quest-transition/native-kills/beginning";
const START: &str = "oteryn:quest-transition/fixture/beginning-kills-start";

#[test]
fn quest_l4_cockroach_three_deaths_advance_only_the_occurrence_gate() -> TestResult {
    run_counter(
        "oteryn:creature.cockroach",
        TRACK,
        KILL,
        START,
        CounterMode::Tutorial,
    )
}

#[test]
fn quest_l4_cockroach_burst_keeps_start_epoch_until_third_death() -> TestResult {
    run_counter(
        "oteryn:creature.cockroach",
        TRACK,
        KILL,
        START,
        CounterMode::TutorialBurst,
    )
}

pub(super) fn catalogue(source: &QuestStateCatalogue) -> TestResult<Arc<QuestStateCatalogue>> {
    let tracks = [GATE, LOG, TRACK]
        .into_iter()
        .map(|key| {
            source.track(key).cloned().unwrap_or(QuestTrack {
                key: key.into(),
                quest: QUEST.into(),
                initial: 0,
                min: 0,
                max: 3,
            })
        })
        .collect();
    let mut transitions = vec![QuestTransition {
        key: START.into(),
        quest: QUEST.into(),
        // Only the previously accepted club/mission is a fixture milestone.
        // No successful death or stage6/hand-in successor is seeded.
        effects: [(GATE, 5), (LOG, 4), (TRACK, 0)]
            .into_iter()
            .map(|(track, value)| QuestEffect {
                track: track.into(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(value),
            })
            .collect(),
        completes: false,
        experience: None,
    }];
    if let Some(kill) = source.transition(KILL) {
        transitions.push(kill.clone());
    }
    Ok(Arc::new(
        QuestStateCatalogue::new("content-1", tracks, transitions).map_err(debug)?,
    ))
}

pub(super) fn assert_goal(tracks: &BTreeMap<String, i64>, count: u64) {
    assert_eq!(
        tracks.get(GATE),
        Some(&if count < 3 { 5 } else { 6 }),
        "only the third native occurrence opens the physical hand-in"
    );
    assert_eq!(
        tracks.get(LOG),
        Some(&4),
        "deaths never claim Santiago's legs/XP reward"
    );
}
