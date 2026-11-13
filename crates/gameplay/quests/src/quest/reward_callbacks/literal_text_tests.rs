use super::*;

#[test]
fn literal_reward_text_reaches_npc_source_validation() -> Result<(), String> {
    let mut document: serde_json::Value =
        serde_json::from_str(DOCUMENTS[0]).map_err(|e| e.to_string())?;
    document["callbacks"][0]["give"] = serde_json::json!({
        "item":"oteryn:item.tibia.i2814", "count":1,"contents":[],"text":"Literal source text.\n"
    });
    parse(&document.to_string())?;
    document["callbacks"][0]["give"]["text"] = "".into();
    assert!(parse(&document.to_string()).is_err());
    Ok(())
}

#[test]
fn literal_reward_text_bounds_and_legacy_serialization() -> Result<(), String> {
    let reward = |text: Option<&str>, count, contents: serde_json::Value| {
        let mut value =
            serde_json::json!({"item":"oteryn:item.tibia.i2814","count":count,"contents":contents});
        if let Some(text) = text {
            value["text"] = text.into();
        }
        serde_json::from_value::<ItemReward>(value).map_err(|e| e.to_string())
    };
    let legacy = reward(None, 1, serde_json::json!([]))?;
    assert!(
        serde_json::to_value(legacy)
            .map_err(|e| e.to_string())?
            .get("text")
            .is_none()
    );
    for text in ["Literal café.\n", &"x".repeat(4000), &"é".repeat(3998)] {
        assert!(reward(Some(text), 1, serde_json::json!([]))?.valid_literal_text());
    }
    for text in ["", "bad\0text", &"x".repeat(4001), &"é".repeat(4000)] {
        assert!(!reward(Some(text), 1, serde_json::json!([]))?.valid_literal_text());
    }
    assert!(!reward(Some("Text"), 2, serde_json::json!([]))?.valid_literal_text());
    assert!(
        !reward(
            Some("Text"),
            1,
            serde_json::json!([{"item":"oteryn:item.tibia.i3031","count":1}])
        )?
        .valid_literal_text()
    );
    Ok(())
}
