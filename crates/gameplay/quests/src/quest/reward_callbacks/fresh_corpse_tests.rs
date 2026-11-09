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
