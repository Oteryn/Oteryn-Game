//! Opt-in exchanges keep legacy carried tools and their definition bytes unchanged.
use super::*;

#[test]
fn explicit_carried_exchange_keeps_exact_selection_and_bounded_costs() -> Result<(), String> {
    let source = load()?
        .into_iter()
        .find(|callback| callback.carried_item.is_some())
        .ok_or("legacy carried callback")?;
    let before = serde_json::to_value(&source).map_err(|error| error.to_string())?;
    assert!(
        before.get("carried_exchange").is_none(),
        "default is absent from existing definition bytes"
    );
    let mut exchange = source.clone();
    exchange.carried_exchange = true;
    exchange.outfit = None;
    exchange.outfits.clear();
    exchange.give = Some(ItemReward {
        text: None,
        additional: Vec::new(),
        premium: false,
        item: "oteryn:item.tibia.i3031".into(),
        count: 5,
        contents: vec![],
        backpack_outputs: vec![],
        key_number: None,
    });
    exchange.take.push(ItemAmount {
        fluid_subtype: None,
        item: "oteryn:item.tibia.i5901".into(),
        count: 1,
    });
    super::parse(&serde_json::json!({"schema":1,"callbacks":[exchange.clone()]}).to_string())?;
    for mutation in 0..6 {
        let mut invalid = exchange.clone();
        match mutation {
            0 => invalid.carried_exchange = false,
            1 => invalid.carried_item = None,
            2 => invalid.take.clear(),
            3 => invalid.take[0].count = 2,
            4 => invalid.give = None,
            _ => invalid.take.push(invalid.take[1].clone()),
        }
        assert!(
            super::parse(&serde_json::json!({"schema":1,"callbacks":[invalid]}).to_string())
                .is_err(),
            "invalid exchange mutation {mutation}"
        );
    }
    assert_eq!(
        serde_json::to_value(source).map_err(|error| error.to_string())?,
        before
    );
    Ok(())
}
