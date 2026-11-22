use super::*;

#[test]
fn m09_wiki_cache_admission_binds_exact_reward_and_once_track() -> Result<(), String> {
    let callbacks = parse(include_str!(
        "../../../../../../content/quests/world-callbacks/m09-treasure.json"
    ))?;
    assert_eq!(callbacks.len(), 3);
    let telescope = callbacks[0]
        .effects
        .item_reward
        .clone()
        .ok_or("telescope reward")?;
    for callback in callbacks {
        assert!(treasure_source::admitted(&callback));
        let mut changed = callback.clone();
        if let Some(reward) = changed.effects.item_reward.as_mut() {
            reward.count += 1;
        } else {
            changed.effects.item_reward = Some(telescope.clone());
        }
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
