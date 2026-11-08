use super::*;

fn native_callback() -> serde_json::Value {
    serde_json::json!({
        "key":"oteryn:quest-reward/native-claim", "quest":"oteryn:quest.authored.kingdom_of_kormarak_quest",
        "npc":"", "keywords":[], "topic":0, "next_topic":0,
        "storage":[{"track":"oteryn:quest-progress/authored/kingdom_of_kormarak_quest/s4", "minimum":0,"maximum":0,"set":1}],
        "take":[], "present":[], "give":{"item":"oteryn:item.tibia.i3031","count":1,"contents":[]},
        "outfit":null, "achievement":null, "reply":["You have found a reward."],
        "sources":["docs/reference/content-prep/engclaim-m34/sources.md"],
        "world_claim":{"key":"oteryn:reward-claim.authored.native","revision":"engclaim-r1","authority_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}
    })
}

#[test]
fn native_claim_reward_has_an_exact_bound_world_owner() -> Result<(), String> {
    let source =
        serde_json::json!({"schema":1,"tracks":[],"callbacks":[native_callback()]}).to_string();
    let document = parse_documents(&[&source])?;
    assert!(gated_binding(&document.callbacks[0])?.is_some());
    Ok(())
}

#[test]
fn native_claim_reward_refuses_npc_routing_and_unbound_identity() -> Result<(), String> {
    for mutation in ["npc", "words", "key", "hash", "carried"] {
        let mut callback = native_callback();
        match mutation {
            "npc" => callback["npc"] = "oteryn:npc.sam".into(),
            "words" => callback["keywords"] = serde_json::json!(["reward"]),
            "key" => callback["world_claim"]["key"] = "oteryn:quest-reward/another-claim".into(),
            "hash" => callback["world_claim"]["authority_sha256"] = "unknown".into(),
            _ => callback["carried_item"] = "oteryn:item.tibia.i3031".into(),
        }
        let source = serde_json::json!({"schema":1,"tracks":[],"callbacks":[callback]}).to_string();
        assert!(parse_documents(&[&source]).is_err(), "{mutation}");
    }
    Ok(())
}
