use oteryn_gameplay_quests::quest::{loader::load_embedded_quest_state, world_callbacks};
use std::collections::BTreeMap;

#[test]
fn qn5_grimvale_entry_uses_registered_stage_without_claiming_completion() -> Result<(), String> {
    let lowered = load_embedded_quest_state("qn5-grimvale-stage").map_err(|e| format!("{e:?}"))?;
    let catalogue = lowered.catalogue();
    let callbacks = world_callbacks::load_for(catalogue)?;
    let callback = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-callback/qn5/grimvale-encounter-entry")
        .ok_or("Grimvale authored stage is not registered at boot")?;
    assert_eq!(callback.quest, "oteryn:quest.grimvale_mini_world_change");
    assert_eq!(callback.trigger, world_callbacks::Trigger::StepIn);
    assert!(callback.requires_world_admission);
    assert!(callback.admission.as_ref().is_some_and(|a| a.premium));
    assert_eq!(callback.world_change.as_deref(), Some("grimvale"));
    assert_eq!(callback.world_requirements.len(), 1);
    assert_eq!(callback.world_requirements[0].appearance, 22761);
    assert_eq!(
        callback.world_requirements[0].position,
        world_callbacks::Position {
            x: 33430,
            y: 31537,
            floor: -11
        }
    );
    assert_eq!(
        callback.selector.position,
        Some(world_callbacks::Position {
            x: 33420,
            y: 31539,
            floor: -10
        })
    );
    let transition = catalogue
        .transition(&callback.key)
        .ok_or("registered transition")?;
    assert!(!transition.completes);
    let track = "oteryn:quest-progress/qn5/grimvale-entry".to_owned();
    let before = BTreeMap::from([(track.clone(), 0)]);
    let changes = catalogue
        .evaluate(transition, &before, 0)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].after, 1);
    assert!(
        catalogue
            .evaluate(transition, &BTreeMap::from([(track, 1)]), 0)
            .is_err()
    );
    Ok(())
}
