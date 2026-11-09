use super::*;
use crate::quest::world_callbacks;

fn document() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../../../content/quests/world-callbacks/questpc5-nomads-cabinet.json"
    ))
    .unwrap_or(serde_json::Value::Null)
}

#[test]
fn nomads_container_reward_keeps_literal_text_on_the_authored_child() -> Result<(), String> {
    let source = document();
    let _: ItemReward =
        serde_json::from_value(source["callbacks"][0]["effects"]["item_reward"].clone())
            .map_err(|error| error.to_string())?;
    let callbacks = world_callbacks::parse(&source.to_string())?;
    let callback = callbacks.first().ok_or("cabinet")?;
    let reward = callback.effects.item_reward.as_ref().ok_or("bag reward")?;
    assert_eq!(reward.item, "oteryn:item.tibia.i2853");
    let text = reward.text.as_ref().ok_or("child text")?;
    assert!(text.root().is_none());
    assert!(text.content(0).is_none());
    assert_eq!(
        text.content(1),
        source["callbacks"][0]["effects"]["item_reward"]["text"]["contents"][0]["text"].as_str()
    );
    assert!(text.content(2).is_none());
    let actual = serde_json::to_value(reward).map_err(|error| error.to_string())?;
    assert_eq!(
        actual["text"],
        source["callbacks"][0]["effects"]["item_reward"]["text"]
    );
    let original = world_callbacks::gated_binding(callback)?.ok_or("bound cabinet")?;
    let mut changed = source.clone();
    changed["callbacks"][0]["effects"]["item_reward"]["text"]["contents"][0]["text"] =
        "Other book".into();
    let changed = world_callbacks::parse(&changed.to_string())?;
    assert_ne!(world_callbacks::gated_binding(&changed[0])?, Some(original));
    Ok(())
}

#[test]
fn nested_reward_text_refuses_ambiguous_or_unbounded_child_plans() {
    for text in [
        serde_json::json!({"contents": []}),
        serde_json::json!({"contents": [{"index": 2, "text": "missing child"}]}),
        serde_json::json!({"contents": [{"index": 1, "text": "a"}, {"index": 1, "text": "b"}]}),
        serde_json::json!({"contents": [{"index": 1, "text": "a"}, {"index": 0, "text": "b"}]}),
        serde_json::json!({"contents": [{"index": 1, "text": ""}]}),
        serde_json::json!({"contents": [{"index": 1, "text": "a\0b"}]}),
        serde_json::json!({"contents": [{"index": 1, "text": "é".repeat(4000)}]}),
        serde_json::json!({"contents": [{"index": 0, "text": "a".repeat(3000)}, {"index": 1, "text": "b".repeat(2000)}]}),
        serde_json::json!({"contents": [{"index": 1, "text": "a", "unknown": 1}]}),
        serde_json::json!({"contents": [{"index": 1, "text": "a"}], "unknown": 1}),
        serde_json::json!("root text on a container"),
    ] {
        let mut invalid = document();
        invalid["callbacks"][0]["effects"]["item_reward"]["text"] = text;
        assert!(world_callbacks::parse(&invalid.to_string()).is_err());
    }
    for (field, value) in [("count", 2), ("fluid_subtype", 1)] {
        let mut invalid = document();
        invalid["callbacks"][0]["effects"]["item_reward"]["contents"][1][field] = value.into();
        assert!(world_callbacks::parse(&invalid.to_string()).is_err());
    }
}

#[test]
fn legacy_literal_reward_string_encoding_stays_unchanged() -> Result<(), String> {
    let reward = ItemReward {
        premium: false,
        item: "oteryn:item.tibia.i2833".into(),
        count: 1,
        contents: Vec::new(),
        text: Some("Original literal".into()),
        additional: Vec::new(),
    };
    assert!(reward.valid_literal_text());
    let value = serde_json::to_value(reward).map_err(|error| error.to_string())?;
    assert_eq!(value["text"], "Original literal");
    Ok(())
}
