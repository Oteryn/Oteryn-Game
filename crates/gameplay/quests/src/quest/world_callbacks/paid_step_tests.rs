use super::*;

fn document() -> Result<serde_json::Value, String> {
    serde_json::from_str(include_str!("paid_step_fixture.json")).map_err(|error| error.to_string())
}

#[test]
fn automatic_step_input_requires_explicit_fenced_native_admission() -> Result<(), String> {
    let document = document()?;
    let callbacks = parse(&document.to_string())?;
    assert!(callbacks[0].item_on_target.as_ref().ok_or("input")?.on_step);
    for mutation in 0..6 {
        let mut invalid = document.clone();
        let callback = &mut invalid["callbacks"][0];
        match mutation {
            0 => callback["item_on_target"]["on_step"] = false.into(),
            1 => callback["trigger"] = "use".into(),
            2 => callback["requires_world_admission"] = false.into(),
            3 => callback["selector"]["native"] = serde_json::Value::Null,
            4 => callback["item_on_target"]["target_replacement"] = 1949.into(),
            _ => callback["item_on_target"]["reusable"] = true.into(),
        }
        assert!(
            parse(&invalid.to_string()).is_err(),
            "accepted mutation {mutation}"
        );
    }
    let mut ordinary_use = document.clone();
    ordinary_use["callbacks"][0]["trigger"] = "use".into();
    ordinary_use["callbacks"][0]["item_on_target"]["on_step"] = false.into();
    let ordinary = parse(&ordinary_use.to_string())?;
    let serialized = serde_json::to_value(&ordinary[0]).map_err(|error| error.to_string())?;
    assert!(
        serialized["item_on_target"].get("on_step").is_none(),
        "existing USE callback serialization and binding hashes stay unchanged"
    );
    Ok(())
}
