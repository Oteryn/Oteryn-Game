use super::*;

#[test]
fn readable_reward_text_is_bounded_and_single_item() -> Result<(), String> {
    let callback = load()?
        .into_iter()
        .find(|c| c.key.ends_with("sam-backpack"))
        .ok_or("callback")?;
    let mut document = serde_json::json!({"schema": 1, "callbacks": [callback]});
    document["callbacks"][0]["give"] = serde_json::json!({
        "item": "oteryn:item.tibia.i2814", "count": 1, "contents": [],
        "text": "The Sacred Casket\nA recovered record."
    });
    assert!(
        parse(&document.to_string()).is_ok(),
        "valid literal text must be admitted"
    );
    for text in [
        String::new(),
        "a\0b".into(),
        "a".repeat(4001),
        "é".repeat(4000),
    ] {
        let mut invalid = document.clone();
        invalid["callbacks"][0]["give"]["text"] = text.into();
        assert!(parse(&invalid.to_string()).is_err());
    }
    let mut multiple = document.clone();
    multiple["callbacks"][0]["give"]["count"] = 2.into();
    assert!(parse(&multiple.to_string()).is_err());
    document["callbacks"][0]["give"]["contents"] = serde_json::json!([
        {"item": "oteryn:item.tibia.i3031", "count": 1}
    ]);
    assert!(parse(&document.to_string()).is_err());
    Ok(())
}
