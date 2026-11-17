use super::*;

#[test]
fn fresh_corpse_mode_is_explicit_bounded_and_reward_bound() -> Result<(), String> {
    let live = load()?
        .into_iter()
        .find(|c| c.creature_target.is_some())
        .ok_or("live creature fixture")?;
    let original = gated_binding(&live)?;
    let mut row = serde_json::to_value(&live).map_err(|e| e.to_string())?;
    assert!(
        row.get("fresh_corpse").is_none(),
        "legacy hashes retain their shape"
    );
    row["fresh_corpse"] = true.into();
    let document =
        |row: &serde_json::Value| serde_json::json!({"schema":1,"callbacks":[row]}).to_string();
    let mut corpse = parse(&document(&row))?;
    assert_ne!(gated_binding(&corpse.remove(0))?, original);
    row["creature_target"] = serde_json::Value::Null;
    row["give"] = serde_json::Value::Null;
    assert!(
        parse(&document(&row)).is_err(),
        "freshness without exact race is forbidden"
    );
    Ok(())
}

#[test]
fn slug_sample_uses_registered_fresh_corpse_and_exact_gatherer() -> Result<(), String> {
    let callback = load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-reward/goblin-merchant/slug-sample")
        .ok_or("missing production Slug sample")?;
    assert!(callback.fresh_corpse);
    assert_eq!(
        callback.creature_target.as_deref(),
        Some("oteryn:creature.slug")
    );
    assert_eq!(
        callback.carried_item.as_deref(),
        Some("oteryn:item.tibia.i17670")
    );
    assert_eq!(callback.take.len(), 1);
    assert_eq!(callback.take[0].item, "oteryn:item.tibia.i17670");
    assert_eq!(callback.take[0].count, 1);
    let output = callback.give.as_ref().ok_or("filled gatherer")?;
    assert_eq!(output.item, "oteryn:item.tibia.i17672");
    assert_eq!(output.count, 1);
    assert!(
        !callback.completes,
        "one odour cannot finish the four-odour task"
    );
    assert_eq!(callback.storage.len(), 1);
    assert_eq!(
        callback.storage[0].track,
        "oteryn:quest-progress/goblin_merchant/slug_sample"
    );
    assert_eq!(callback.storage[0].set, 1);
    let loaded = super::super::loader::load_embedded_quest_state("content-1")
        .map_err(|e| format!("{e:?}"))?;
    let track = loaded
        .catalogue()
        .track(&callback.storage[0].track)
        .ok_or("registered sample track")?;
    assert_eq!(track.quest, callback.quest);
    assert_eq!((track.initial, track.min, track.max), (0, 0, 1));
    assert!(loaded.catalogue().transition(&callback.key).is_some());
    Ok(())
}
