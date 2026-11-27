use super::TestResult;

#[test]
fn between_the_lines_phillip_accepts_the_investigation() -> TestResult {
    super::qengine_rewards_fixtures::scenario(super::between_the_lines_fixtures::PHILLIP_SCENARIO)
}

#[test]
fn between_the_lines_ink_traces_load_as_source_checkpoints() -> TestResult {
    use oteryn_gameplay_quests::quest::{loader, reward_callbacks, world_callbacks};
    let loaded = loader::load_embedded_quest_state("content-1").map_err(super::fixture::debug)?;
    let quests = loaded.catalogue();
    let accept = reward_callbacks::load_for(quests)?
        .into_iter()
        .find(|row| row.key.ends_with("/between-the-lines-phillip-accept"))
        .ok_or("Phillip's investigation")?;
    let ink = world_callbacks::load_for(quests)?
        .into_iter()
        .find(|row| row.key.ends_with("/between-the-lines-ink-trace"))
        .ok_or("ink trace checkpoint")?;
    assert_eq!(ink.trigger, world_callbacks::Trigger::StepIn);
    assert_eq!(ink.selector.ordinal, Some(0));
    assert_eq!(ink.selector.position_aliases.len(), 6);
    for position in ink
        .selector
        .position
        .into_iter()
        .chain(ink.selector.position_aliases.iter().copied())
    {
        super::classic_quest_source_fixtures::committed_attrs(
            (position.x, position.y, position.floor),
            50881,
        )?;
    }
    let transition = quests.transition(&ink.key).ok_or("ink transition")?;
    assert_eq!(transition.effects.len(), 2);
    assert_eq!(ink.storage[0].track, accept.storage[0].track);
    assert_eq!(ink.storage[0].minimum, 1);
    assert!(ink.storage[0].preserve);
    for checkpoint in [&accept.storage[0], &ink.storage[1]] {
        let initial = quests
            .track(&checkpoint.track)
            .ok_or("checkpoint track")?
            .initial;
        assert!((checkpoint.minimum..=checkpoint.maximum).contains(&initial));
    }
    assert_eq!(ink.storage[1].set, 1);
    Ok(())
}
