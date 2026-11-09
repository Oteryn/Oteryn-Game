use super::*;

#[test]
fn native_plain_item_plans_keep_vocation_guards() -> Result<(), NpcRuleError> {
    let mut document: serde_json::Value =
        serde_json::from_str(include_str!("test-fixtures/hero_greeting.json"))
            .map_err(|error| NpcRuleError::Json(error.to_string()))?;
    document["rules"][0]["guards"] = serde_json::json!([
        {"kind":"vocation","value":"oteryn:vocation.knight"},
    ]);
    document["rules"][0]["effects"]
        .as_array_mut()
        .ok_or_else(|| NpcRuleError::Invalid("fixture effects".into()))?
        .push(serde_json::json!({"kind":"give_item","item":"oteryn:item.3031","count":1}));
    let source = NpcRuleCatalog::from_json_documents(&[&document.to_string()])?;
    assert!(source.requires_vocation("oteryn:npc.strange_pipe"));
    assert!(!source.requires_vocation("oteryn:npc.unrelated"));
    let inventory = NpcInventorySnapshot::default();
    let tracks = BTreeMap::new();
    let outfits = BTreeMap::new();
    let mut input = NpcRuleInput {
        npc: "oteryn:npc.strange_pipe",
        trigger: NpcRuleTrigger::Greet,
        message: "hi",
        topic: 0,
        tracks: &tracks,
        inventory: &inventory,
        now_epoch_seconds: 1_700_000_000,
        calendar_month: 11,
        level: 100,
        vocation: "oteryn:vocation.druid",
        sex: 0,
        outfits: &outfits,
    };
    assert!(source.plan_item_exchange(&input)?.is_none());
    input.vocation = "oteryn:vocation.knight";
    let plan = source
        .plan_item_exchange(&input)?
        .ok_or_else(|| NpcRuleError::Invalid("missing vocation-gated item plan".into()))?;
    assert_eq!(
        plan.inventory_after.items.get(&NpcItemKey {
            item: "oteryn:item.3031".into(),
            subtype: None,
        }),
        Some(&1)
    );
    assert_eq!(plan.track_changes.len(), 1);
    Ok(())
}
