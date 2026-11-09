//! M07: source Lion questline advances max(old + 1, 2), without unchecked overflow.
use super::*;
use std::collections::BTreeMap;

#[test]
fn m07_offering_counter_initializes_unset_progress_then_preserves_later_steps() -> Result<(), String>
{
    let operation: reward_callbacks::StateOperation = serde_json::from_str(
        r#"{
        "track":"oteryn:quest-progress/m07/test-counter","from":{"kind":"any"},
        "effect":{"kind":"add_at_least","value":1,"minimum":2}
    }"#,
    )
    .map_err(|e| e.to_string())?;
    let owner = "oteryn:quest.m07_counter_fixture";
    let transition = QuestTransition {
        key: "oteryn:quest-transition/m07/counter".into(),
        quest: owner.into(),
        effects: vec![operation.quest_effect()],
        completes: false,
        experience: None,
        world_change: None,
    };
    let catalogue = QuestStateCatalogue::new(
        "m07-counter",
        vec![QuestTrack {
            key: operation.track.clone(),
            quest: owner.into(),
            initial: 0,
            min: -1,
            max: i64::MAX,
        }],
        vec![transition.clone()],
    )
    .map_err(|e| format!("{e:?}"))?;
    for (before, after) in [(-1, 2), (0, 2), (1, 2), (2, 3), (3, 4), (100, 101)] {
        let changes = catalogue
            .evaluate(
                &transition,
                &BTreeMap::from([(operation.track.clone(), before)]),
                0,
            )
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(changes.first().ok_or("counter change")?.after, after);
    }
    assert!(
        catalogue
            .evaluate(
                &transition,
                &BTreeMap::from([(operation.track, i64::MAX)]),
                0
            )
            .is_err()
    );
    Ok(())
}
