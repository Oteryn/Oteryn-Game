use super::*;

#[test]
fn literal_reward_text_reaches_world_source_validation() -> Result<(), String> {
    let mut callback = load()?
        .into_iter()
        .find(|c| c.effects.item_reward.is_some())
        .ok_or("existing World item reward fixture")?;
    callback.effects.item_reward = Some(serde_json::from_value(serde_json::json!({
        "item":"oteryn:item.tibia.i2814", "count":1,"contents":[],"text":"Literal source text.\n"
    })).map_err(|e| e.to_string())?);
    let document =
        |callback: &Callback| serde_json::json!({"schema":1,"callbacks":[callback]}).to_string();
    parse(&document(&callback))?;
    callback.effects.item_reward.as_mut().ok_or("reward")?.text = Some("".into());
    assert!(parse(&document(&callback)).is_err());
    Ok(())
}
