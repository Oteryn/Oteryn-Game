//! Regression coverage for the existing remains callback and its shared quest identity.
use oteryn_gameplay_quests::quest::{QuestRefusal, QuestTrackChange, loader, world_callbacks};
use std::collections::BTreeMap;

const CALLBACK: &str = "oteryn:quest-callback/the-lost-brother-remains";
const TRACK: &str = "oteryn:quest-progress/quest/u10_80/the_lost_brother_quest";

#[test]
fn lost_brother_existing_remains_requires_active_stage_and_refuses_reentry() -> Result<(), String> {
    let callback = world_callbacks::load()?
        .into_iter()
        .find(|callback| callback.key == CALLBACK)
        .ok_or("existing Lost Brother remains callback missing")?;
    assert_eq!(callback.quest, "oteryn:quest.adventurers_guild");
    assert_eq!(callback.trigger, world_callbacks::Trigger::StepIn);
    assert_eq!(
        callback.selector.position,
        Some(world_callbacks::Position {
            x: 32959,
            y: 32674,
            floor: -4
        })
    );
    let loaded = loader::load_embedded_quest_state("qcx5-lost-brother-existing")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let transition = catalogue
        .transition(CALLBACK)
        .ok_or("existing remains transition missing")?;
    assert_eq!(transition.quest, callback.quest);
    assert!(!transition.completes);
    assert!(transition.experience.is_none());
    assert_eq!(
        catalogue.evaluate(transition, &BTreeMap::new(), 0),
        Err(QuestRefusal::StageMismatch)
    );
    for stage in [-1, 0, 2, 3] {
        assert_eq!(
            catalogue.evaluate(transition, &BTreeMap::from([(TRACK.into(), stage)]), 0),
            Err(QuestRefusal::StageMismatch),
            "stage {stage} cannot discover the remains again"
        );
    }
    assert_eq!(
        catalogue.evaluate(transition, &BTreeMap::from([(TRACK.into(), 1)]), 0),
        Ok(vec![QuestTrackChange {
            track: TRACK.into(),
            before: 1,
            after: 2
        }])
    );
    Ok(())
}
