//! M39: absence is an explicit bounded inventory guard, not a positive hand-in.
use super::*;

#[test]
fn m39_npc_sample_reward_uses_the_existing_quest_xp_obligation()
-> Result<(), Box<dyn std::error::Error>> {
    let callback = load()?.into_iter().next().ok_or("source callback")?;
    let mut source = serde_json::to_value(callback)?;
    source["experience"] = 1000.into();
    let parsed: RewardCallback = serde_json::from_value(source)?;
    assert_eq!(primary_transition(&parsed)?.experience, Some(1000));
    Ok(())
}

#[test]
fn m39_reward_callback_accepts_an_explicit_item_absence_guard()
-> Result<(), Box<dyn std::error::Error>> {
    let callback = load()?.into_iter().next().ok_or("source callback")?;
    let mut source = serde_json::to_value(callback)?;
    source["inventory"] = serde_json::json!([
        {"item": "oteryn:item.tibia.i131", "minimum": 0, "maximum": 0}
    ]);
    let parsed: RewardCallback = serde_json::from_value(source)?;
    let encoded = serde_json::to_value(parsed)?;
    assert_eq!(encoded["inventory"][0]["minimum"], 0);
    assert_eq!(encoded["inventory"][0]["maximum"], 0);
    Ok(())
}

#[test]
fn m39_inventory_ranges_distinguish_absence_exact_count_and_overflow() {
    let item = "oteryn:item.tibia.i131".to_owned();
    for (minimum, maximum) in [(0, 0), (1, 1), (2, 5), (0, u32::MAX)] {
        let rule = InventoryRequirement {
            item: item.clone(),
            minimum,
            maximum,
        };
        assert_eq!(rule.holds(&BTreeMap::new()), minimum == 0);
        for quantity in [0, 1, 2, 5, 6, u64::from(u32::MAX), u64::from(u32::MAX) + 1] {
            let facts = BTreeMap::from([(item.clone(), quantity)]);
            assert_eq!(
                rule.holds(&facts),
                (u64::from(minimum)..=u64::from(maximum)).contains(&quantity)
            );
        }
    }
}

#[test]
fn m39_quantity_terms_bind_the_reward_but_refusal_speech_does_not()
-> Result<(), Box<dyn std::error::Error>> {
    let mut callback = load()?
        .into_iter()
        .find(|c| c.minimum_level.is_none())
        .ok_or("legacy callback")?;
    assert_eq!(
        gated_binding(&callback)?,
        None,
        "legacy callbacks retain their encoding"
    );
    callback.inventory = vec![InventoryRequirement {
        item: "oteryn:item.tibia.i131".into(),
        minimum: 0,
        maximum: 0,
    }];
    let absent = gated_binding(&callback)?.ok_or("quantity binding")?;
    callback.failure_reply = vec!["You cannot deliver that sample.".into()];
    assert_eq!(gated_binding(&callback)?, Some(absent));
    callback.inventory[0].minimum = 1;
    callback.inventory[0].maximum = 1;
    assert_ne!(gated_binding(&callback)?, Some(absent));
    Ok(())
}
