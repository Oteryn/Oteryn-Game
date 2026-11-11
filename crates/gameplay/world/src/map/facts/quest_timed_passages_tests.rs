//! M07 source deadlines run through the real Channel-owned map projection.
use super::*;

fn entry(appearance_id: u16) -> EntryFacts {
    EntryFacts {
        definition: MapDefinition::Item(NonZeroU32::MIN),
        terrain_kind: None,
        appearance_id,
        blocks_projectile: false,
        pickupable: false,
        bound: true,
        count: 1,
        sub_type: 0,
    }
}

#[test]
fn m07_remote_wall_and_lever_restore_on_distinct_source_deadlines() -> Result<(), String> {
    let facts = BundleFacts::default();
    let lever = entry(2772);
    let wall = entry(1270);
    let pagoda = entry(2074);
    for (key, before) in [(1, lever), (2, wall), (4, lever), (5, pagoda)] {
        assert!(facts.apply_quest_callback(key, 0, before, &[], &[], false, None));
    }
    let walls = QuestCallbackProjection::TimedPassages {
        due: 7_000_000,
        entries: vec![QuestTimedPassage {
            key: 2,
            expected_revision: 1,
            before: wall,
            after: None,
            passable: false,
        }],
        source_reset: Some((8_000_000, lever)),
    };
    assert!(facts.quest_timed_passages_ready(1, 1, &walls));
    assert_eq!(facts.object_revision(2), 1, "preflight must not publish");
    assert_eq!(facts.next_movement_decay_time(), None);
    assert!(facts.apply_quest_callback(1, 1, entry(2773), &[], &[], false, Some(&walls)));
    let pagodas = QuestCallbackProjection::TimedPassages {
        due: 15_000_000,
        entries: vec![QuestTimedPassage {
            key: 5,
            expected_revision: 1,
            before: pagoda,
            after: Some(entry(2075)),
            passable: true,
        }],
        source_reset: None,
    };
    assert!(facts.apply_quest_callback(4, 1, lever, &[], &[], false, Some(&pagodas)));
    assert!(facts.base_hidden(2));
    assert_eq!(facts.expire_movement_items(6_999_999), 0);
    assert_eq!(facts.expire_movement_items(7_000_000), 1);
    assert!(!facts.base_hidden(2));
    assert_eq!(facts.object_revision(2), 3);
    assert_eq!(facts.object_revision(1), 2);
    assert_eq!(facts.expire_movement_items(8_000_000), 1);
    assert_eq!(facts.object_revision(1), 3);
    assert_eq!(facts.expire_movement_items(14_999_999), 0);
    assert_eq!(facts.expire_movement_items(15_000_000), 1);
    let states = facts.callbacks.lock().map_err(|_| "callbacks lock")?;
    assert_eq!(
        states.changed.get(&5).map(|(_, f)| f.appearance_id),
        Some(2074)
    );
    assert!(
        !states.passable.contains_key(&5),
        "restore original collision override"
    );
    Ok(())
}

#[test]
fn m07_remote_reset_cannot_overwrite_newer_revision_or_partially_publish() -> Result<(), String> {
    let facts = BundleFacts::default();
    let wall = entry(1270);
    let lever = entry(2772);
    for key in 1..=3 {
        assert!(facts.apply_quest_callback(key, 0, wall, &[], &[], false, None));
    }
    let change = |key, expected_revision| QuestTimedPassage {
        key,
        expected_revision,
        before: wall,
        after: None,
        passable: false,
    };
    let stale = QuestCallbackProjection::TimedPassages {
        due: 7,
        entries: vec![change(2, 1), change(3, 0)],
        source_reset: None,
    };
    assert!(!facts.quest_timed_passages_ready(1, 1, &stale));
    assert!(!facts.apply_quest_callback(1, 1, lever, &[], &[], false, Some(&stale)));
    assert_eq!(facts.object_revision(1), 1);
    assert!(!facts.base_hidden(2));
    assert_eq!(facts.next_movement_decay_time(), None);
    let valid = QuestCallbackProjection::TimedPassages {
        due: 7,
        entries: vec![change(2, 1)],
        source_reset: None,
    };
    assert!(facts.apply_quest_callback(1, 1, lever, &[], &[], false, Some(&valid)));
    assert!(facts.apply_quest_callback(2, 2, entry(1025), &[], &[], false, None));
    assert_eq!(facts.expire_movement_items(7), 0);
    assert!(
        facts.base_hidden(2),
        "stale timer cannot change newer visibility"
    );
    assert_eq!(facts.object_revision(2), 3);
    Ok(())
}
