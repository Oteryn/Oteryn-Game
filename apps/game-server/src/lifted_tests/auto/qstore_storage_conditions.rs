//! QSTORE conditions use the production callback authoring and quest evaluator.
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, world_callbacks};

#[test]
fn qstore_storage_bit_conditions_lower_and_refuse_false_gates() -> Result<(), String> {
    let mut callback = world_callbacks::load()?
        .into_iter()
        .find(|callback| callback.key == oteryn_gameplay_quests::quest::rotten_blood::SACRIFICE)
        .ok_or("Rotten Blood callback")?;
    callback.native_rule = None;
    callback.storage.clear();
    callback.state_operations = serde_json::from_value(serde_json::json!([{
        "track":"oteryn:quest-progress/qstore/flags",
        "from":{"kind":"bits_set","mask":5},
        "effect":{"kind":"add","value":0}
    }]))
    .map_err(|e| e.to_string())?;
    let transition = world_callbacks::primary_transition(&callback)?;
    let catalogue = QuestStateCatalogue::new(
        "qstore",
        vec![oteryn_gameplay_quests::quest::QuestTrack {
            key: callback.state_operations[0].track.clone(),
            quest: callback.quest.clone(),
            initial: 0,
            min: -1,
            max: i64::MAX,
        }],
        vec![transition],
    )
    .map_err(|e| format!("{e:?}"))?;
    for (value, allowed) in [
        (0, false),
        (1, false),
        (4, false),
        (5, true),
        (7, true),
        (-1, true),
    ] {
        let values = [(callback.state_operations[0].track.clone(), value)]
            .into_iter()
            .collect();
        assert_eq!(
            catalogue
                .evaluate(
                    catalogue.transition(&callback.key).ok_or("transition")?,
                    &values,
                    0
                )
                .is_ok(),
            allowed
        );
    }
    for kind in ["bits_set", "bits_clear"] {
        for mask in [0, -1] {
            let operation: oteryn_gameplay_quests::quest::reward_callbacks::StateOperation =
                serde_json::from_value(serde_json::json!({
                    "track":"oteryn:quest-progress/qstore/flags",
                    "from":{"kind":kind,"mask":mask},"effect":{"kind":"add","value":0}
                }))
                .map_err(|e| e.to_string())?;
            assert!(!operation.valid());
        }
    }
    Ok(())
}
