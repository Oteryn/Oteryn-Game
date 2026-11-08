//! Children of the Revolution grease action retains the source player voice.
use oteryn_gameplay_quests::quest::world_callbacks;

#[test]
fn qm46_grease_action_binds_public_player_speech() -> Result<(), String> {
    let callbacks = world_callbacks::load()?;
    let callback = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-callback/children-grease")
        .ok_or("missing grease callback")?;
    let wire = serde_json::to_value(callback).map_err(|e| e.to_string())?;
    assert_eq!(
        wire["effects"]["player_speech"],
        "Due to being extra greasy, the leavers can now be moved."
    );
    assert_eq!(callback.selector.action, Some(8013));
    assert_eq!(
        callback.item_on_target.as_ref().map(|i| i.item.as_str()),
        Some("oteryn:item.tibia.i10189")
    );
    assert!(
        callback
            .storage
            .iter()
            .any(|s| s.track.ends_with("/mission04")
                && s.minimum == 3
                && s.maximum == 3
                && s.set == 4)
    );
    assert!(
        callback
            .storage
            .iter()
            .any(|s| s.track.ends_with("/questline") && s.set == 14)
    );
    Ok(())
}

#[test]
fn qm46_grease_transition_requires_mission_three_and_preserves_atomic_progress()
-> Result<(), String> {
    use oteryn_gameplay_quests::quest::{QuestRefusal, loader};
    use std::collections::BTreeMap;
    let loaded = loader::load_embedded_quest_state("qm46-grease").map_err(|e| format!("{e:?}"))?;
    let catalogue = loaded.catalogue();
    let transition = catalogue
        .transition("oteryn:quest-callback/children-grease")
        .ok_or("grease transition")?;
    let mission = "oteryn:quest-progress/quest/u8_54/children_of_the_revolution/mission04";
    let questline = "oteryn:quest-progress/quest/u8_54/children_of_the_revolution/questline";
    for value in [-1, 0, 2, 4, 5, 6] {
        assert_eq!(
            catalogue.evaluate(transition, &BTreeMap::from([(mission.into(), value)]), 0),
            Err(QuestRefusal::StageMismatch)
        );
    }
    let changes = catalogue
        .evaluate(
            transition,
            &BTreeMap::from([(mission.into(), 3), (questline.into(), 13)]),
            0,
        )
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(changes.len(), 2);
    assert!(
        changes
            .iter()
            .any(|c| c.track == mission && c.before == 3 && c.after == 4)
    );
    assert!(
        changes
            .iter()
            .any(|c| c.track == questline && c.before == 13 && c.after == 14)
    );
    Ok(())
}
