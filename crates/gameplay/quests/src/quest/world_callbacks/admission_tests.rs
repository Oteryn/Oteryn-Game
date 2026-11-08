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

#[test]
fn live_admission_is_bound_even_without_the_optional_world_flag() -> Result<(), String> {
    let mut callback = load()?
        .into_iter()
        .find(|c| !c.storage.is_empty() && c.native_rule.is_none())
        .ok_or("personal callback")?;
    callback.requires_world_admission = false;
    callback.opening_only = false;
    callback.admission = None;
    assert_eq!(
        gated_binding(&callback)?,
        None,
        "legacy unguarded bytes keep their binding"
    );
    callback.admission = Some(Admission {
        promotion: true,
        ..Default::default()
    });
    let promoted = gated_binding(&callback)?.ok_or("live admission must bind its predicate")?;
    callback.admission = Some(Admission {
        premium: true,
        ..Default::default()
    });
    assert_ne!(
        Some(promoted),
        gated_binding(&callback)?,
        "different live predicates cannot reuse a receipt binding"
    );
    Ok(())
}
