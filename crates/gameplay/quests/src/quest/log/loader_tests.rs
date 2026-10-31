#![allow(clippy::expect_used)]

use super::*;

#[test]
fn quest_log_content_loads_real_journals_and_records_unbound_entries() {
    let lowered =
        super::super::super::loader::load_embedded_quest_state("content-1").expect("state");
    let loaded = load_embedded_quest_log(&lowered).expect("real journal catalogue");
    assert!(loaded.content.active().quests().len() > 80);
    assert!(
        loaded
            .exclusions
            .iter()
            .all(|entry| entry.reason == "no admitted progress track")
    );
    // W15c now binds the Grizzly points/counters to the existing source missions.
    assert!(
        loaded
            .exclusions
            .iter()
            .all(|entry| entry.mission.as_deref() != Some("paw_and_fur_hunting_elite"))
    );
    let first = &loaded.content.active().quests()[0];
    assert_eq!(first.key, "oteryn:quest.a_father_s_burden_quest");
    assert_eq!(first.missions[0].name, "The Birthday Presents");
    assert!(
        matches!(&first.missions[0].journal, super::super::QuestJournal::PerStage(entries) if entries.len() == 2)
    );
    let source: serde_json::Value = serde_json::from_str(QUESTS).expect("source");
    // Source missions retain their order, except for the native Monk pilgrimage journal.
    for quest in source["quests"].as_array().expect("quests") {
        let key = quest["identity"]["key"]
            .as_str()
            .expect("key")
            .split_once(":quest/")
            .map(|(_, key)| format!("oteryn:quest.{key}"))
            .expect("source key");
        let Some(admitted) = loaded
            .content
            .active()
            .quests()
            .iter()
            .find(|q| q.key == key)
        else {
            assert!(
                loaded
                    .exclusions
                    .iter()
                    .any(|entry| entry.quest == key && entry.mission.is_none())
            );
            continue;
        };
        let expected: Vec<_> = if key == super::super::super::monk_shrines::QUEST {
            // The native Monk journal replaces The Invitation and Dawnport Shrines with
            // The Three-Fold Path plus the ten mainland shrines, in pilgrimage order.
            vec![
                "The Three-Fold Path",
                "Shrine of Guidance",
                "Shrine of Tranquility",
                "Shrine of Respect",
                "Shrine of Legacy",
                "Shrine of Empathy",
                "Shrine of Harmony",
                "Shrine of Power",
                "Shrine of Knowledge",
                "Shrine of Serenity",
                "Shrine of Eternity",
            ]
        } else {
            quest["missions"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|mission| {
                    !loaded.exclusions.iter().any(|entry| {
                        entry.quest == key && entry.mission.as_deref() == mission["key"].as_str()
                    })
                })
                .map(|mission| mission["name"].as_str().expect("name"))
                .collect()
        };
        assert_eq!(
            admitted
                .missions
                .iter()
                .map(|m| m.name.as_str())
                .collect::<Vec<_>>(),
            expected
        );
    }
}

#[test]
fn quest_log_content_rejects_corrupt_captured_prose() {
    let lowered =
        super::super::super::loader::load_embedded_quest_state("content-1").expect("state");
    let mut texts: serde_json::Value = serde_json::from_str(TEXTS).expect("texts");
    texts["texts"][0]["text"] = "corrupted".into();
    assert!(parse(QUESTS, &texts.to_string(), &lowered).is_err());
}
