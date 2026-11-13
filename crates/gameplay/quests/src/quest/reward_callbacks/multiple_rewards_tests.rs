use super::ItemReward;

#[test]
fn qmech_multiple_independent_rewards_keep_their_exact_outputs()
-> Result<(), Box<dyn std::error::Error>> {
    let value = serde_json::json!({
        "premium": false, "item": "oteryn:item.tibia.i3412", "count": 1,
        "contents": [],
        "additional": [{"item": "oteryn:item.tibia.i266", "count": 7}]
    });
    let reward: ItemReward = serde_json::from_value(value)?;
    let encoded = serde_json::to_value(reward)?;
    assert_eq!(encoded["item"], "oteryn:item.tibia.i3412");
    assert_eq!(encoded["additional"][0]["item"], "oteryn:item.tibia.i266");
    assert_eq!(encoded["additional"][0]["count"], 7);
    assert_eq!(encoded["contents"], serde_json::json!([]));
    Ok(())
}

#[test]
fn qmech_multiple_independent_rewards_reject_unsupported_sibling_metadata() -> Result<(), String> {
    let mut document: serde_json::Value =
        serde_json::from_str(super::DOCUMENTS[0]).map_err(|e| e.to_string())?;
    let give = &mut document["callbacks"][0]["give"];
    *give = serde_json::json!({"item":"oteryn:item.tibia.i3412","count":1,
        "additional":[{"item":"oteryn:item.tibia.i266","count":7}]});
    assert!(super::parse(&document.to_string()).is_ok());
    document["callbacks"][0]["give"]["additional"][0]["fluid_subtype"] = 1.into();
    assert!(super::parse(&document.to_string()).is_err());
    document["callbacks"][0]["give"]["additional"] = serde_json::json!([
        {"item":"oteryn:item.tibia.i266","count":0}]);
    assert!(super::parse(&document.to_string()).is_err());
    Ok(())
}
