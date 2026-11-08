use super::*;

#[test]
fn world_admission_predicates_require_personal_fenced_player_callbacks() -> Result<(), String> {
    let callback = load()?
        .into_iter()
        .find(|c| !c.storage.is_empty() && !c.reversible)
        .ok_or("personal callback")?;
    let mut value = serde_json::to_value(&callback).map_err(|e| e.to_string())?;
    value["admission"] =
        serde_json::json!({"regular_blessings":true,"promotion":true,"premium":true});
    let document =
        |value: &serde_json::Value| serde_json::json!({"schema":1,"callbacks":[value]}).to_string();
    assert!(
        parse(&document(&value)).is_ok(),
        "live admission predicates must be authorable"
    );
    value["storage"] = serde_json::json!([]);
    value["state_ops"] = serde_json::json!([]);
    assert!(
        parse(&document(&value)).is_err(),
        "admission cannot bypass the personal writer"
    );
    Ok(())
}
