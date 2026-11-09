use super::*;

#[test]
fn capture_policy_binds_live_occurrences_and_preserves_legacy_hashes() -> Result<(), String> {
    let live = load()?
        .into_iter()
        .find(|c| c.creature_target.is_some() && !c.fresh_corpse)
        .ok_or("live callback")?;
    let original = gated_binding(&live)?;
    let mut row = serde_json::to_value(&live).map_err(|e| e.to_string())?;
    assert!(row.get("capture_creature").is_none());
    row["capture_creature"] = true.into();
    let document =
        |row: &serde_json::Value| serde_json::json!({"schema":1,"callbacks":[row]}).to_string();
    let capture = parse(&document(&row))?.remove(0);
    assert_ne!(gated_binding(&capture)?, original);
    row["fresh_corpse"] = true.into();
    assert!(
        parse(&document(&row)).is_err(),
        "capture cannot consume a corpse"
    );
    row["fresh_corpse"] = false.into();
    row["creature_target"] = serde_json::Value::Null;
    assert!(
        parse(&document(&row)).is_err(),
        "capture needs an exact native creature"
    );
    Ok(())
}
