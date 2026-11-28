use oteryn_gameplay_quests::quest::{loader::load_embedded_quest_state, reward_callbacks};

fn wayfarer_addon_callbacks() -> Result<Vec<reward_callbacks::RewardCallback>, String> {
    let loaded =
        load_embedded_quest_state("wayfarer-addons").map_err(|error| format!("{error:?}"))?;
    let callbacks: Vec<_> = reward_callbacks::offered_for(loaded.catalogue())?
        .into_iter()
        .filter(|row| row.key.starts_with("oteryn:quest-reward/l1-wayfarer-"))
        .collect();
    assert_eq!(callbacks.len(), 4);
    Ok(callbacks)
}

#[test]
fn wayfarer_first_and_final_hand_ins_require_current_premium() -> Result<(), String> {
    for callback in wayfarer_addon_callbacks()? {
        assert!(callback.requires_premium, "{}", callback.key);
    }
    Ok(())
}

#[test]
fn wayfarer_first_and_final_hand_ins_grant_only_the_addon() -> Result<(), String> {
    for callback in wayfarer_addon_callbacks()? {
        assert!(callback.addon_only, "{}", callback.key);
        assert!(callback.requires_outfits.is_empty(), "{}", callback.key);
        assert_eq!(
            callback.requires_completed_quests,
            ["oteryn:quest.the_spirit_will_get_you_quest"],
            "{}",
            callback.key
        );
    }
    Ok(())
}
