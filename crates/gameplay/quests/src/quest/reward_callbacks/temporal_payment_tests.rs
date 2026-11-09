use super::*;

fn document() -> serde_json::Value {
    let track = "oteryn:quest-progress/npcfix/travora_ticket_purchased";
    let quest = "oteryn:quest.npcfix_ticket_fixture";
    serde_json::json!({"schema":1,
    "tracks":[{"writable":true,"key":track,"quest":quest,"initial":0,"min":0,"max":i64::MAX,
        "sources":["docs/reference/tibia-manual/products.md:162"]}],
    "callbacks":[{
        "key":"oteryn:quest-reward/npcfix/ticket_purchase", "quest":quest,
        "npc":"oteryn:npc.m38_ticket_fixture", "keywords":["yes"],"topic":1,"next_topic":0,
        "storage":[], "state_ops":[{"track":track,"from":{"kind":"elapsed_at_least","seconds":86400},"effect":{"kind":"set_now"}}],
        "requires":[],"take":[],"give":null,"outfit":null,"achievement":null,
        "fee":{"gold":1000,"include_bank":true},
        "reply":["Your ticket is valid for24 hours."],
        "sources":["docs/reference/tibia-manual/products.md:162"]
    }]})
}

#[test]
fn m38_travel_temporal_purchase_admits_exact_fee_and_cooldown_stamp() -> Result<(), String> {
    let callbacks = parse(&document().to_string())?;
    assert_eq!(callbacks.len(), 1);
    assert_eq!(callbacks[0].fee.as_ref().map(|fee| fee.gold), Some(1000));
    assert_eq!(callbacks[0].state_ops[0].effect, StateMutation::SetNow);
    assert_eq!(
        callbacks[0].state_ops[0].from,
        StateCondition::ElapsedAtLeast { seconds: 86400 }
    );
    Ok(())
}

#[test]
fn m38_travel_temporal_purchase_refuses_unguarded_or_free_refreshes() {
    let variants = [
        ("fee", serde_json::Value::Null),
        ("fee", serde_json::json!({"gold":0,"include_bank":true})),
        ("state_ops", serde_json::json!([])),
        ("completes", serde_json::json!(true)),
    ];
    for (field, value) in variants {
        let mut data = document();
        data["callbacks"][0][field] = value;
        assert!(
            parse(&data.to_string()).is_err(),
            "unsafe purchase field {field}"
        );
    }
    for condition in [
        serde_json::json!({"kind":"any"}),
        serde_json::json!({"kind":"elapsed_at_least","seconds":0}),
        serde_json::json!({"kind":"elapsed_at_least","seconds":366 * 86400 + 1}),
    ] {
        let mut data = document();
        data["callbacks"][0]["state_ops"][0]["from"] = condition;
        assert!(parse(&data.to_string()).is_err());
    }
}
