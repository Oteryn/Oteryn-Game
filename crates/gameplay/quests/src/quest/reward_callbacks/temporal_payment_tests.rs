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

fn ticket_catalogue(callback: &RewardCallback) -> Result<QuestStateCatalogue, String> {
    QuestStateCatalogue::new(
        "m38-ticket-binding",
        vec![QuestTrack {
            key: callback.state_ops[0].track.clone(),
            quest: callback.quest.clone(),
            initial: 0,
            min: 0,
            max: i64::MAX,
        }],
        vec![primary_transition(callback)?],
    )
    .map_err(|error| format!("{error:?}"))
}

#[test]
fn m38_travel_ticket_fee_terms_change_the_primary_definition_hash() -> Result<(), String> {
    let callback = parse(&document().to_string())?
        .into_iter()
        .next()
        .ok_or("ticket callback")?;
    let original = ticket_catalogue(&callback)?.definition_hash(&callback.quest);
    for fee in [
        MonetaryFee {
            gold: 1001,
            include_bank: true,
        },
        MonetaryFee {
            gold: 1000,
            include_bank: false,
        },
    ] {
        let mut changed = callback.clone();
        changed.fee = Some(fee);
        assert_ne!(
            ticket_catalogue(&changed)?.definition_hash(&changed.quest),
            original,
            "changing payment terms must supersede the timestamp grant's quest pin"
        );
    }
    let mut display = callback.clone();
    display.reply = vec!["Different ticket wording.".into()];
    display.sources = vec!["Different witness location.".into()];
    assert_eq!(
        ticket_catalogue(&display)?.definition_hash(&display.quest),
        original
    );
    Ok(())
}

#[test]
fn m38_travel_bound_ticket_uses_transaction_time_and_keeps_the_cooldown() -> Result<(), String> {
    let callback = parse(&document().to_string())?
        .into_iter()
        .next()
        .ok_or("ticket callback")?;
    let catalogue = ticket_catalogue(&callback)?;
    let transition = catalogue
        .transition(&callback.key)
        .ok_or("ticket transition")?;
    assert!(matches!(
        transition.effects[0].effect,
        QuestEffectKind::SetNowWithBinding { .. }
    ));
    let track = callback.state_ops[0].track.clone();
    let bought = catalogue
        .evaluate(transition, &BTreeMap::new(), 200_000)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(bought[0].after, 200_000);
    let stored = BTreeMap::from([(track, 200_000)]);
    for now in [199_999, 200_000, 286_399] {
        assert_eq!(
            catalogue.evaluate(transition, &stored, now),
            Err(super::super::QuestRefusal::StageMismatch)
        );
    }
    assert_eq!(
        catalogue
            .evaluate(transition, &stored, 286_400)
            .map_err(|error| format!("{error:?}"))?[0]
            .after,
        286_400
    );
    assert!(
        super::super::admission_binding::validate(transition, false).is_err(),
        "an unregistered caller cannot supply a bound clock operation"
    );
    Ok(())
}

#[test]
fn m38_travel_bound_clock_preserves_legacy_effect_encodings() {
    let mut legacy = Vec::new();
    QuestEffectKind::SetNow.encode(&mut legacy);
    assert_eq!(legacy, [3]);
    let mut conditional = Vec::new();
    QuestEffectKind::SetWhenAtLeast {
        other_effect: 0,
        minimum: 1,
        value: 1,
    }
    .encode(&mut conditional);
    assert_eq!(conditional[0], 11);
    let mut bound = Vec::new();
    QuestEffectKind::SetNowWithBinding { binding: [7; 32] }.encode(&mut bound);
    assert_eq!(bound, [&[12][..], &[7; 32][..]].concat());
    assert_ne!(bound, legacy);
}
