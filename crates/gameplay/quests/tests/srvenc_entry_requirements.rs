//! M03 source entrances: Crystal actions_time_machine.lua:1-11,26-49;
//! chosen requirement evidence: quest definitions 00200-00299:1 record67,
//! 00000-00099:1 record89. The donor supplies movement, the chosen recipes gates.
use oteryn_gameplay_quests::quest::{loader::load_embedded_quest_state, world_callbacks};

#[test]
fn m03_production_entrances_bind_live_requirements_and_real_stage_counters() -> Result<(), String> {
    let book = load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
    let callbacks = world_callbacks::load_for(book.catalogue())?;
    let entry = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-callback/qm03/anniversary-25095")
        .ok_or("M03 time machine production entry")?;
    assert_eq!(entry.minimum_level, Some(20));
    let requirements = entry.character_requirements.as_ref().ok_or("live guards")?;
    assert!(requirements.premium && requirements.promoted);
    assert_eq!(requirements.blessings.len(), 5);
    // The central part has its authored key; the other eight parts are aliases
    // under the same admission and stage owner.
    let entries: Vec<_> = callbacks
        .iter()
        .filter(|c| c.quest == entry.quest && c.admission.is_some())
        .collect();
    assert_eq!(entries.len(), 9);
    assert!(entries.iter().all(|c| c.requires_world_admission
        && c.trigger == world_callbacks::Trigger::Use
        && c.minimum_level == entry.minimum_level
        && c.admission == entry.admission
        && c.character_requirements == entry.character_requirements
        && c.storage == entry.storage));
    let appearances = entries
        .iter()
        .map(|c| {
            c.selector
                .native
                .as_ref()
                .map(|native| native.appearance)
                .ok_or("native time machine part")
        })
        .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
    assert_eq!(appearances, (25091..=25099).collect());
    let returns: Vec<_> = callbacks
        .iter()
        .filter(|c| {
            c.key == "oteryn:quest-callback/m03/25-years/time-machine-return"
                || c.key
                    .starts_with("oteryn:quest-callback/qm03/anniversary-return-")
        })
        .collect();
    assert_eq!(returns.len(), 9);
    // The authored returns keep the shared stage counter without requiring
    // any entrance predicate or resetting the value already earned.
    assert!(returns.iter().all(|c| c.quest == entry.quest
        && c.admission.is_none()
        && c.character_requirements.is_none()
        && c.minimum_level.is_none()
        && !c.requires_world_admission
        && c.trigger == world_callbacks::Trigger::Use
        && c.storage.len() == 1
        && c.storage[0].track == entry.storage[0].track
        && c.storage[0].preserve
        && !c.storage[0].increment
        && !c.storage[0].at_least));
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
    assert!(poi.requires_world_admission);
    for callback in entries
        .into_iter()
        .chain(std::iter::once(poi))
        .chain(returns)
    {
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
        assert!(
            !book
                .catalogue()
                .transition(&callback.key)
                .ok_or("canonical entrance or return transition")?
                .completes
        );
    }
    Ok(())
}
