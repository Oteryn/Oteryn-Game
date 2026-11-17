//! State-catalogue checks for existing mission actions and movements.
use oteryn_gameplay_quests::quest::{QuestRefusal, QuestTrackChange, loader};
use std::collections::BTreeMap;

fn verify_guard(action: &str, track: &str, other: &str) -> Result<(), String> {
    let loaded = loader::load_embedded_quest_state("qcx5-existing-mission-guards")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let transition = catalogue
        .transition(action)
        .ok_or_else(|| format!("missing {action}"))?;
    let mut progress = BTreeMap::from([(other.to_owned(), 1)]);
    assert_eq!(
        catalogue.evaluate(transition, &progress, 0),
        Err(QuestRefusal::StageMismatch),
        "progress on {other} must not qualify {action}"
    );
    progress.insert(track.to_owned(), 1);
    let changes = catalogue
        .evaluate(transition, &progress, 0)
        .map_err(|error| format!("{action} with its own prerequisite: {error:?}"))?;
    assert_eq!(
        changes,
        vec![QuestTrackChange {
            track: track.to_owned(),
            before: 1,
            after: 2
        }]
    );
    for change in changes {
        progress.insert(change.track, change.after);
    }
    assert_eq!(
        catalogue.evaluate(transition, &progress, 0),
        Err(QuestRefusal::StageMismatch),
        "{action} must refuse a second activation after its recorded change"
    );
    Ok(())
}

#[test]
fn secret_service_movement_uses_its_own_agency_progress() -> Result<(), String> {
    verify_guard(
        "oteryn:quest-transition/quest/u8_1/secret_service/avinmission03/movement_1",
        "oteryn:quest-progress/quest/u8_1/secret_service/avinmission03",
        "oteryn:quest-progress/quest/u8_1/secret_service/tbimission03",
    )
}

#[test]
fn shadows_mission_nine_action_uses_its_own_mission_progress() -> Result<(), String> {
    verify_guard(
        "oteryn:quest-transition/crystalserver/quest/u8_5/shadows_of_yalahar/mission09/action_1",
        "oteryn:quest-progress/crystalserver/quest/u8_5/shadows_of_yalahar/mission09",
        "oteryn:quest-progress/crystalserver/quest/u8_5/shadows_of_yalahar/mission11",
    )
}
