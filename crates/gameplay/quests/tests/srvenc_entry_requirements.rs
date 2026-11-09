//! M03 source entrances: Crystal actions_time_machine.lua:1-11,26-49;
//! chosen requirement evidence: quest definitions 00200-00299:1 record67,
//! 00000-00099:1 record89. The donor supplies movement, the chosen recipes gates.
use oteryn_gameplay_quests::quest::{loader::load_embedded_quest_state, world_callbacks};

#[test]
fn m03_production_entrances_bind_live_requirements_and_real_stage_counters() -> Result<(), String> {
    let callbacks = world_callbacks::load()?;
    let book = load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
    let entry = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-callback/qm03/anniversary-25095")
        .ok_or("M03 time machine production entry")?;
    assert_eq!(entry.minimum_level, Some(20));
    let requirements = entry.character_requirements.as_ref().ok_or("live guards")?;
    assert!(requirements.premium && requirements.promoted);
    assert_eq!(requirements.blessings.len(), 5);
    assert_eq!(
        callbacks
            .iter()
            .filter(|c| c.key.starts_with("oteryn:quest-callback/qm03/anniversary"))
            .count(),
        9
    );
    let returns: Vec<_> = callbacks
        .iter()
        .filter(|c| {
            c.key
                .starts_with("oteryn:quest-callback/m03/time-machine-return-")
        })
        .collect();
    assert_eq!(returns.len(), 9);
    assert!(returns.iter().all(|c| c.character_requirements.is_none()
        && c.minimum_level.is_none()
        && c.storage.is_empty()));
    let poi = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-callback/m03/pits-of-inferno-entry")
        .ok_or("M03 Pits production entry")?;
    assert_eq!(poi.minimum_level, Some(80));
    assert!(
        poi.character_requirements
            .as_ref()
            .is_some_and(|r| r.premium && !r.promoted && r.blessings.is_empty())
    );
    for callback in [entry, poi] {
        assert!(callback.requires_world_admission);
        let track = book
            .catalogue()
            .track(&callback.storage[0].track)
            .ok_or("canonical chosen stage counter")?;
        assert_eq!(track.initial, 0);
        assert_eq!(track.max, 1);
        assert_eq!(
            book.catalogue().transition(&callback.key),
            Some(&world_callbacks::primary_transition(callback)?)
        );
    }
    Ok(())
}
