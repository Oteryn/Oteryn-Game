//! A spent material may own a new reward epoch; a reusable tool may not.
use super::*;

#[test]
fn spent_material_counter_can_own_repeatable_crafting_rewards() -> Result<(), String> {
    let mut callback = load()?
        .into_iter()
        .find(|c| c.item_on_target.is_some())
        .ok_or("carried material fixture")?;
    callback.key = "oteryn:quest-callback/qready1-cursed-grind".into();
    callback.quest = "oteryn:quest.the_cursed_crystal".into();
    callback.native_rule = None;
    callback.admission = None;
    callback.world_change = None;
    callback.reversible = false;
    callback.opening_only = false;
    callback.requires_world_admission = false;
    callback.minimum_level = None;
    callback.offerings.clear();
    callback.world_requirements.clear();
    callback.state_ops.clear();
    callback.storage = vec![Storage {
        track: "oteryn:quest-progress/qready1-craft/grind".into(),
        minimum: 0,
        maximum: 1_000_000,
        set: 1,
        preserve: false,
        at_least: false,
        increment: true,
    }];
    callback.item_on_target = Some(ItemOnTarget {
        item: "oteryn:item.tibia.i16122".into(),
        alternatives: vec![],
        count: 1,
        replacement: None,
        target_replacement: None,
        reusable: false,
    });
    callback.effects = serde_json::from_value(serde_json::json!({
        "item_reward":{"item":"oteryn:item.tibia.i21507","count":1,"contents":[]},
        "item_reward_cycle":callback.storage[0].track
    }))
    .map_err(|e| e.to_string())?;
    let document = |c: &Callback| serde_json::json!({"schema":1,"callbacks":[c]}).to_string();
    parse(&document(&callback))?;
    for edit in 0..5 {
        let mut bad = callback.clone();
        match edit {
            0 => bad.item_on_target.as_mut().ok_or("input")?.reusable = true,
            1 => bad.item_on_target = None,
            2 => bad.storage[0].increment = false,
            3 => bad.storage[0].set = 2,
            _ => {
                bad.effects.item_reward.as_mut().ok_or("reward")?.item =
                    "oteryn:item.tibia.i16122".into()
            }
        }
        assert!(
            parse(&document(&bad)).is_err(),
            "free or unbounded craft shape {edit}"
        );
    }
    Ok(())
}
