use super::*;

#[test]
fn m41_source_achievement_survives_storage_and_item_planning() -> Result<(), NpcRuleError> {
    let track = "oteryn:quest-progress/m41-fixture/stage";
    for with_item in [false, true] {
        let mut effects = vec![
            serde_json::json!({"kind":"set","track":track,"value":1}),
            serde_json::json!({"kind":"achievement","name":"Bastard"}),
        ];
        if with_item {
            effects.push(
                serde_json::json!({"kind":"give_item","item":"oteryn:item.tibia.i3031","count":1}),
            );
        }
        let text = serde_json::json!({"schema":1,"quest":"oteryn:quest.m41_fixture",
            "tracks":[{"key":track,"quest":"oteryn:quest.m41_fixture","initial":0,"min":0,"max":1,
                "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/eldoran_the_ambitious.lua:90"]}],
            "rules":[{"key":"oteryn:quest-npc/m41_fixture/final","quest":"oteryn:quest.m41_fixture",
                "npc":"oteryn:npc.fixture","trigger":"say","keywords":["yes"],
                "guards":[{"kind":"storage","track":track,"op":"eq","value":0}],
                "effects":effects,"reply":["Done."],
                "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/eldoran_the_ambitious.lua:90"]}]}
        ).to_string();
        let source = NpcRuleCatalog::from_json_documents(&[&text])?;
        let tracks = BTreeMap::new();
        let outfits = BTreeMap::new();
        let inventory = NpcInventorySnapshot::default();
        let input = NpcRuleInput {
            player_name: None,
            npc: "oteryn:npc.fixture",
            trigger: NpcRuleTrigger::Say,
            message: "yes",
            topic: 0,
            tracks: &tracks,
            inventory: &inventory,
            now_epoch_seconds: 0,
            calendar_month: 1,
            level: 1,
            vocation: "",
            premium_current: false,
            sex: 0,
            outfits: &outfits,
        };
        let plan = if with_item {
            source.plan_item_exchange(&input)?
        } else {
            source.plan_storage(&input)?
        }
        .ok_or_else(|| NpcRuleError::MissingNativeResolution("M41 finale".into()))?;
        assert_eq!(plan.track_changes[0].after, 1);
        assert!(plan.effects.iter().any(|effect| matches!(effect,
            NpcRuleEffect::Achievement { name } if name == "Bastard")));
        assert_eq!(plan.inventory_after.items.len(), usize::from(with_item));
    }
    Ok(())
}

#[test]
fn m41_stag_authored_finale_preserves_sword_achievement_and_progress() -> Result<(), NpcRuleError> {
    let source = NpcRuleCatalog::runtime()?;
    let track = "oteryn:quest-progress/crystalserver/quest/u15_20/the_order_of_the_stag/questline";
    let tracks = BTreeMap::from([(track.into(), 12)]);
    let inventory = NpcInventorySnapshot::default();
    let outfits = BTreeMap::new();
    let input = NpcRuleInput {
        player_name: None,
        npc: "oteryn:npc.eldoran_the_ambitious",
        trigger: NpcRuleTrigger::Say,
        message: "news",
        topic: 0,
        tracks: &tracks,
        inventory: &inventory,
        now_epoch_seconds: 0,
        calendar_month: 1,
        level: 1,
        vocation: "",
        premium_current: false,
        sex: 0,
        outfits: &outfits,
    };
    let plan = source
        .plan_item_exchange(&input)?
        .ok_or_else(|| NpcRuleError::MissingNativeResolution("Stag authored finale".into()))?;
    assert_eq!(
        plan.selected_rule_keys[0],
        "oteryn:quest-npc/the_order_of_the_stag/eldoran_the_ambitious/news-reward"
    );
    assert!(
        plan.track_changes
            .iter()
            .any(|change| change.key == track && change.before == 12 && change.after == 13)
    );
    assert!(plan.effects.iter().any(|effect| matches!(effect,
        NpcRuleEffect::Achievement { name } if name == "Bastard")));
    assert_eq!(
        plan.inventory_after.items.get(&NpcItemKey {
            item: "oteryn:item.tibia.i52824".into(),
            subtype: None
        }),
        Some(&1)
    );
    Ok(())
}
