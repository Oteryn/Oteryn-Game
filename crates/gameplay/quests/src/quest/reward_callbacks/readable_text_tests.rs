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

#[test]
fn personal_map_marker_is_bounded_and_pins_reward_terms() -> Result<(), String> {
    let callback = load()?
        .into_iter()
        .find(|c| c.key.ends_with("sam-backpack"))
        .ok_or("callback")?;
    let mut document = serde_json::json!({"schema":1,"callbacks":[callback]});
    document["callbacks"][0]["map_mark"] = serde_json::json!({"position":{"x":31948,"y":31924,"floor":-7},"kind":"cross","text":"Treasure"});
    assert!(parse(&document.to_string()).is_ok());
    let parsed: RewardCallback =
        serde_json::from_value(document["callbacks"][0].clone()).map_err(|e| e.to_string())?;
    let before = gated_binding(&parsed)?.ok_or("marker terms")?;
    document["callbacks"][0]["map_mark"]["position"]["x"] = 31949.into();
    let changed: RewardCallback =
        serde_json::from_value(document["callbacks"][0].clone()).map_err(|e| e.to_string())?;
    assert_ne!(gated_binding(&changed)?, Some(before));
    document["callbacks"][0]["map_mark"]["text"] = "x".repeat(129).into();
    assert!(parse(&document.to_string()).is_err());
    Ok(())
}

#[test]
fn sterling_commits_the_source_treasure_mark_with_both_progress_writes() -> Result<(), String> {
    let callbacks = load()?;
    let callback = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/targuna/sterling-mark-treasure")
        .ok_or("Sterling callback")?;
    let mark = callback.map_mark.as_ref().ok_or("personal marker")?;
    assert_eq!(
        (mark.position.x, mark.position.y, mark.position.floor),
        (31948, 31924, -7)
    );
    assert_eq!(mark.kind, QuestMapMarkIcon::Cross);
    assert_eq!(mark.text, "Treasure");
    assert!(
        callback
            .storage
            .iter()
            .any(|s| s.track.ends_with("/mission")
                && s.minimum == 3
                && s.maximum == 3
                && s.set == 4)
    );
    assert!(
        callback
            .storage
            .iter()
            .any(|s| s.track.ends_with("/treasure_marked")
                && s.minimum == 0
                && s.maximum == 0
                && s.set == 1)
    );
    assert!(
        callback
            .requires
            .iter()
            .any(|s| s.track.ends_with("/morla_done") && s.minimum == 1 && s.maximum == 1)
    );
    Ok(())
}
