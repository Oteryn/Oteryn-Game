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
    assert!(
        loaded
            .exclusions
            .iter()
            .any(|entry| entry.mission.as_deref() == Some("paw_and_fur_hunting_elite"))
    );
    let first = &loaded.content.active().quests()[0];
    assert_eq!(first.key, "oteryn:quest.a_father_s_burden_quest");
    assert_eq!(first.missions[0].name, "The Birthday Presents");
    assert!(
        matches!(&first.missions[0].journal, super::super::QuestJournal::PerStage(entries) if entries.len() == 2)
    );
    let source: serde_json::Value = serde_json::from_str(QUESTS).expect("source");
    // Every source mission is either admitted in its original position or explicitly excluded.
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
        let expected: Vec<_> = quest["missions"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|mission| {
                !loaded.exclusions.iter().any(|entry| {
                    entry.quest == key && entry.mission.as_deref() == mission["key"].as_str()
                })
            })
            .map(|mission| mission["name"].as_str().expect("name"))
            .collect();
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
