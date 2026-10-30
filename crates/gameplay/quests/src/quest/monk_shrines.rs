//! The mainland pilgrimage reads its ten authoritative flags, never the donor counter.

use std::collections::BTreeMap;

pub const QUEST: &str = "oteryn:quest.the_way_of_the_monk_quest";
pub const START_TRACK: &str = "oteryn:quest-progress/quest/u14_15/the_way_of_the_monk/quest_line";
pub const PILGRIMAGE_TRACK: &str =
    "oteryn:quest-progress/quest/u15_00/the_way_of_the_monk/questline";
pub const FINAL_TRACK: &str = "oteryn:quest-progress/monk-pilgrimage/final-report";
pub const INVITE: &str = "oteryn:quest-transition/monk-pilgrimage/invitation";
pub const START: &str = "oteryn:quest-transition/monk-pilgrimage/start";
pub const FINISH: &str = "oteryn:quest-transition/monk-pilgrimage/finish";

/// The final reward is derived by the Wheel owner from the once-only report flag.
pub fn final_points() -> Option<u32> {
    #[derive(serde::Deserialize)]
    struct Rewards {
        final_promotion_points: u32,
    }
    let data: Rewards = serde_json::from_str(include_str!(
        "../../../../../content/quests/monk-pilgrimage.json"
    ))
    .ok()?;
    Some(data.final_promotion_points)
}

pub(super) fn journal() -> super::log::QuestLogQuest {
    use super::log::{JournalText, QuestJournal, QuestLogKind, QuestLogMission, QuestLogQuest};
    let mut missions = vec![QuestLogMission {
        name: "The Three-Fold Path".into(), track: PILGRIMAGE_TRACK.into(), start_value: 0, end_value: 4,
        journal: QuestJournal::PerStage(BTreeMap::from([
            (0, JournalText::Text("Ambassador Manop has invited you to learn about the Merudri. Seek Enpa-Deia Pema in Blue Valley to begin your pilgrimage.".into())),
            (3, JournalText::Template(vec![super::log::JournalPiece::Text("Honour the ten mainland shrines in order. Completed shrines: ".into()), super::log::JournalPiece::Track(COUNT_TRACK.into()), super::log::JournalPiece::Text("/10. Return to Enpa-Deia Pema after Eternity.".into())])),
            (4, JournalText::Text("You completed the pilgrimage and reported to Enpa-Deia Pema. The Merudri have granted your final promotion points.".into())),
        ])),
    }];
    missions.extend(NAMES.iter().map(|name| QuestLogMission {
        name: format!("Shrine of {}{}", name[..1].to_ascii_uppercase(), &name[1..]),
        track: format!("{FLAG_PREFIX}{name}"), start_value: 1, end_value: 1,
        journal: QuestJournal::Fixed(JournalText::Text(format!("You honoured the Shrine of {name}. Your shrine rewards are earned; make room in your backpack and use the shrine again if an item reward is pending."))),
    }));
    QuestLogQuest {
        key: QUEST.into(),
        kind: QuestLogKind::Storyline,
        shown_in_quest_log: true,
        start: Some((START_TRACK.into(), 1)),
        hide_when_completed: false,
        missions,
    }
}

pub(super) fn extend(
    catalogue: super::QuestStateCatalogue,
) -> Result<super::QuestStateCatalogue, String> {
    use super::{QuestComparison, QuestEffect, QuestEffectKind, QuestTrack, QuestTransition};
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    tracks.push(QuestTrack {
        key: FINAL_TRACK.into(),
        quest: QUEST.into(),
        initial: 0,
        min: 0,
        max: 1,
    });
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    let effect = |key: &str, low, high, set| QuestEffect {
        track: key.into(),
        from: QuestComparison::Between(low, high),
        effect: QuestEffectKind::Set(set),
    };
    for (key, effects, completes) in [
        (INVITE, vec![effect(START_TRACK, 0, 0, 1)], false),
        (
            START,
            vec![
                effect(START_TRACK, 0, 1, 1),
                effect(PILGRIMAGE_TRACK, 0, 2, 3),
            ],
            false,
        ),
        (
            FINISH,
            vec![
                effect(PILGRIMAGE_TRACK, 3, 3, 4),
                effect(FINAL_TRACK, 0, 0, 1),
                effect(&format!("{FLAG_PREFIX}eternity"), 1, 1, 1),
            ],
            true,
        ),
    ] {
        transitions.push(QuestTransition {
            key: key.into(),
            quest: QUEST.into(),
            effects,
            completes,
            experience: None,
        });
    }
    super::QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|e| format!("{e:?}"))
}

pub const COUNT_TRACK: &str =
    "oteryn:quest-progress/quest/u14_15/the_way_of_the_monk/shrines_count";
pub const FLAG_PREFIX: &str =
    "oteryn:quest-progress/quest/u15_00/the_way_of_the_monk/main_shrines/";
pub const NAMES: [&str; 10] = [
    "guidance",
    "tranquility",
    "respect",
    "legacy",
    "empathy",
    "harmony",
    "power",
    "knowledge",
    "serenity",
    "eternity",
];

/// Only exact completed flags count. The legacy counter is never evidence of completion.
#[must_use]
pub fn completed_count(tracks: &BTreeMap<String, i64>) -> i64 {
    NAMES
        .iter()
        .map(|name| i64::from(tracks.get(&format!("{FLAG_PREFIX}{name}")) == Some(&1)))
        .sum()
}

/// Only Respect through Power grant the offensive bonus.
#[must_use]
pub fn offensive_count(tracks: &BTreeMap<String, i64>) -> u8 {
    NAMES[2..7].iter().fold(0, |count, name| {
        count + u8::from(tracks.get(&format!("{FLAG_PREFIX}{name}")) == Some(&1))
    })
}

/// Overlay the compatibility counter on a coherent owner read or committed session copy.
pub fn derive_count(tracks: &mut BTreeMap<String, i64>) {
    if tracks.contains_key(COUNT_TRACK) || tracks.keys().any(|key| key.starts_with(FLAG_PREFIX)) {
        tracks.insert(COUNT_TRACK.into(), completed_count(tracks));
    }
}

#[cfg(test)]
mod tests {
    use crate::quest::{QuestRefusal, loader, world_callbacks};
    use std::collections::BTreeMap;

    #[test]
    fn monk_shrine_rewards_declare_xp_items_and_explicit_spell_learning() -> Result<(), String> {
        let data: serde_json::Value =
            serde_json::from_str(world_callbacks::EMBEDDED).map_err(|e| e.to_string())?;
        let callbacks = data["callbacks"].as_array().ok_or("callbacks")?;
        let experience = [
            300, 2500, 5000, 10000, 15000, 30000, 60000, 75000, 150000, 500000,
        ];
        let items = [
            Some(50267),
            Some(50271),
            None,
            Some(50269),
            Some(50273),
            Some(50274),
            Some(50272),
            None,
            Some(50268),
            None,
        ];
        for (index, name) in super::NAMES.iter().enumerate() {
            let key = format!("oteryn:quest-callback/monk-{name}");
            let callback = callbacks
                .iter()
                .find(|c| c["key"].as_str() == Some(&key))
                .ok_or(name.to_string())?;
            assert_eq!(
                callback["experience"].as_i64(),
                Some(experience[index]),
                "{name} XP"
            );
            assert_eq!(
                callback["monk_reward"]["item_source_id"].as_u64(),
                items[index]
            );
        }
        Ok(())
    }

    #[test]
    fn monk_shrine_offence_uses_only_the_five_exact_completed_flags() {
        let mut tracks = BTreeMap::from([(super::COUNT_TRACK.into(), 999)]);
        for (index, name) in super::NAMES.iter().enumerate() {
            let key = format!("{}{name}", super::FLAG_PREFIX);
            tracks.insert(key.clone(), 2);
            assert_eq!(
                super::offensive_count(&tracks),
                index.saturating_sub(2).min(5) as u8
            );
            tracks.insert(key, 1);
            assert_eq!(
                super::offensive_count(&tracks),
                (index + 1).saturating_sub(2).min(5) as u8
            );
        }
        assert_eq!(super::completed_count(&tracks), 10);
        assert_eq!(super::offensive_count(&tracks), 5);
    }

    #[test]
    fn monk_shrine_count_never_trusts_the_old_counter_or_other_tracks() {
        let mut tracks = BTreeMap::from([
            (super::COUNT_TRACK.into(), 11),
            (format!("{}respect", super::FLAG_PREFIX), 1),
            (format!("{}legacy", super::FLAG_PREFIX), 2),
            (format!("{}unknown", super::FLAG_PREFIX), 1),
        ]);
        super::derive_count(&mut tracks);
        assert_eq!(tracks.get(super::COUNT_TRACK), Some(&1));
        tracks.remove(&format!("{}respect", super::FLAG_PREFIX));
        super::derive_count(&mut tracks);
        assert_eq!(tracks.get(super::COUNT_TRACK), Some(&0));
    }

    #[test]
    fn monk_shrines_real_callbacks_are_ordered_and_repeat_safe() -> Result<(), String> {
        let callbacks = world_callbacks::load()?;
        let shrines: Vec<_> = callbacks
            .iter()
            .filter(|c| c.quest == "oteryn:quest.the_way_of_the_monk_quest")
            .collect();
        assert_eq!(shrines.len(), 10);
        let lowered =
            loader::load_embedded_quest_state("content-1").map_err(|e| format!("{e:?}"))?;
        let catalogue = lowered.catalogue();
        let old_action = catalogue
            .transition(
                "oteryn:quest-transition/quest/u14_15/the_way_of_the_monk/shrines_count/action_1",
            )
            .ok_or("legacy binding")?;
        assert_eq!(
            catalogue.evaluate(old_action, &BTreeMap::new(), 0),
            Err(QuestRefusal::NotSupported)
        );
        let mut progress = BTreeMap::new();
        let start = catalogue
            .transition(super::START)
            .ok_or("pilgrimage start")?;
        for change in catalogue
            .evaluate(start, &progress, 0)
            .map_err(|e| format!("{e:?}"))?
        {
            progress.insert(change.track, change.after);
        }
        for name in [
            "guidance",
            "tranquility",
            "respect",
            "legacy",
            "empathy",
            "harmony",
            "power",
            "knowledge",
            "serenity",
            "eternity",
        ] {
            let callback = shrines.iter().find(|c| c.key.ends_with(name)).ok_or(name)?;
            let transition = catalogue.transition(&callback.key).ok_or(name)?;
            if name != "guidance" {
                assert_eq!(
                    catalogue.evaluate(transition, &BTreeMap::new(), 0),
                    Err(QuestRefusal::StageMismatch),
                    "{name} cannot skip the preceding shrine"
                );
            }
            let changes = catalogue
                .evaluate(transition, &progress, 0)
                .map_err(|e| format!("{name}: {e:?}"))?;
            for change in changes {
                progress.insert(change.track, change.after);
            }
            assert_eq!(
                catalogue.evaluate(transition, &progress, 0),
                Err(QuestRefusal::StageMismatch),
                "{name} cannot complete twice"
            );
        }
        assert_eq!(super::completed_count(&progress), 10);
        progress.insert(super::COUNT_TRACK.into(), 999);
        super::derive_count(&mut progress);
        assert_eq!(progress.get(super::COUNT_TRACK), Some(&10));
        Ok(())
    }
}
