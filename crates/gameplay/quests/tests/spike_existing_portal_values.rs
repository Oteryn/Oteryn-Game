//! Regression for the existing Oteryn Spike portal callbacks' value-free behavior.
use oteryn_gameplay_quests::quest::world_callbacks;

#[test]
fn spike_existing_portals_have_distinct_selectors_and_no_value_or_progress_writes()
-> Result<(), String> {
    let callbacks = world_callbacks::load()?;
    let portals: Vec<_> = callbacks
        .iter()
        .filter(|callback| {
            callback
                .key
                .starts_with("oteryn:quest-callback/spike-portal-")
        })
        .collect();
    assert_eq!(portals.len(), 17);
    for action in 4227..=4243 {
        let key = format!("oteryn:quest-callback/spike-portal-{action}");
        let callback = portals
            .iter()
            .find(|callback| callback.key == key)
            .ok_or_else(|| format!("existing Spike portal missing: {key}"))?;
        assert_eq!(callback.trigger, world_callbacks::Trigger::StepIn);
        assert_eq!(callback.selector.action, Some(action));
        assert!(callback.teleport.is_some());
        assert!(!callback.has_state_writes());
        assert!(callback.offerings.is_empty());
        assert!(callback.item_on_target.is_none());
        assert!(callback.monk_reward.is_none());
        assert!(callback.experience.is_none());
    }
    Ok(())
}
