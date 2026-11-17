use super::*;

#[test]
fn m09_wiki_cache_admission_binds_exact_reward_and_once_track() -> Result<(), String> {
    let callbacks = parse(include_str!(
        "../../../../../../content/quests/world-callbacks/m09-treasure.json"
    ))?;
    assert_eq!(callbacks.len(), 2);
    for callback in callbacks {
        assert!(treasure_source::admitted(&callback));
        let mut changed = callback.clone();
        changed.effects.item_reward.as_mut().ok_or("reward")?.count += 1;
        assert!(!treasure_source::admitted(&changed));
        let mut changed = callback.clone();
        changed.storage[0].set = 0;
        assert!(!treasure_source::admitted(&changed));
        let mut changed = callback.clone();
        changed.selector.position.as_mut().ok_or("position")?.y += 1;
        assert!(!treasure_source::admitted(&changed));
        let mut changed = callback;
        changed.sources.clear();
        assert!(!treasure_source::admitted(&changed));
    }
    Ok(())
}
