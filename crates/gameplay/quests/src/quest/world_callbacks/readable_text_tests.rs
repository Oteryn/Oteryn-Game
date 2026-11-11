use super::*;

#[test]
fn world_readable_rewards_use_the_same_text_bounds() -> Result<(), String> {
    let mut callback = load()?
        .into_iter()
        .find(|c| c.effects.item_reward.is_some())
        .ok_or("world reward")?;
    let reward = callback.effects.item_reward.as_mut().ok_or("item reward")?;
    reward.item = "oteryn:item.tibia.i2814".into();
    reward.count = 1;
    reward.contents.clear();
    reward.text = Some("An authored record.\nSecond line.".into());
    let mut document = serde_json::json!({"schema": 1, "callbacks": [callback]});
    assert!(parse(&document.to_string()).is_ok());
    for text in ["a\0b".to_owned(), "é".repeat(4000)] {
        document["callbacks"][0]["effects"]["item_reward"]["text"] = text.into();
        assert!(parse(&document.to_string()).is_err());
    }
    Ok(())
}
