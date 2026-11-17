//! Donor odds and boundary draws shared with the durable NPC exchange.
use oteryn_gameplay_quests::quest::npc_rules::NpcRuleEffect;
use oteryn_gameplay_quests::quest::npc_rules::random_outcome::SANDRA_LOTTERY;

#[test]
fn sandra_lottery_missing_input_refuses_without_drawing() {
    let called = std::cell::Cell::new(false);
    assert_eq!(
        SANDRA_LOTTERY.effects(0, |_| {
            called.set(true);
            0
        }),
        None
    );
    assert!(!called.get());
}

#[test]
fn sandra_lottery_resolves_exact_input_and_selected_output() {
    let take = NpcRuleEffect::TakeItem {
        item: "oteryn:item.tibia.i5957".into(),
        count: 1,
        subtype: None,
        optional: false,
    };
    assert_eq!(
        SANDRA_LOTTERY.effects(1, |total| {
            assert_eq!(total, 98);
            0
        }),
        Some(vec![
            take.clone(),
            NpcRuleEffect::GiveItem {
                item: "oteryn:item.tibia.i5958".into(),
                count: 1,
                subtype: None,
            }
        ])
    );
    assert_eq!(SANDRA_LOTTERY.effects(1, |_| 97), Some(vec![take]));
    assert_eq!(SANDRA_LOTTERY.effects(1, |_| 98), None);
}

#[test]
fn sandra_lottery_odds_equal_crystal_201189f8() {
    assert_eq!(SANDRA_LOTTERY.total_weight(), Some(98));
    assert_eq!(SANDRA_LOTTERY.outcomes[0].weight, 1);
    assert_eq!(SANDRA_LOTTERY.outcomes[1].weight, 97);
    assert_eq!(SANDRA_LOTTERY.input, "oteryn:item.tibia.i5957");
    assert_eq!(
        SANDRA_LOTTERY.draw(0),
        Some(Some("oteryn:item.tibia.i5958"))
    );
    for draw in 1..98 {
        assert_eq!(SANDRA_LOTTERY.draw(draw), Some(None));
    }
    assert_eq!(SANDRA_LOTTERY.draw(98), None);
}

#[test]
fn sandra_lottery_server_rng_is_seeded_and_receipt_bound() {
    use crate::durability::quest_state::npc_program::sandra_lottery::draw;
    let binding = [7; 33];
    let draws: Vec<_> = (0..=255_u8).map(|seed| draw([seed; 32], binding)).collect();
    assert!(draws.contains(&Some(0)), "seeded win must be reachable");
    assert!(
        draws
            .iter()
            .any(|value| value.is_some_and(|value| value > 0))
    );
    for seed in 0..=255_u8 {
        assert_eq!(draw([seed; 32], binding), draws[usize::from(seed)]);
    }
}

#[test]
fn sandra_lottery_distinct_receipts_have_distinct_rng_streams() {
    use crate::durability::quest_state::npc_program::sandra_lottery::draw;
    let first: Vec<_> = (0..=255_u8).map(|seed| draw([seed; 32], [1; 33])).collect();
    let second: Vec<_> = (0..=255_u8).map(|seed| draw([seed; 32], [2; 33])).collect();
    assert_ne!(first, second);
    assert!(
        first
            .iter()
            .chain(&second)
            .all(|value| value.is_some_and(|value| value < 98))
    );
}

#[test]
fn sandra_lottery_dialogue_is_registered_for_production() -> super::TestResult {
    use oteryn_gameplay_quests::quest::npc_rules::NpcRuleCatalog;
    let source = NpcRuleCatalog::runtime()?;
    let rule = source
        .programs()
        .iter()
        .flat_map(|program| &program.document.rules)
        .find(|rule| rule.key == "oteryn:quest-npc/sandra/lottery")
        .ok_or("Sandra lottery dialogue missing")?;
    assert_eq!(rule.npc, "oteryn:npc.sandra");
    assert_eq!(rule.keywords, ["lottery"]);
    assert!(source.item_exchange_keys(&rule.npc).is_some());
    Ok(())
}

#[test]
fn sandra_lottery_claim_is_repeatable_per_ticket_and_stable_on_replay() {
    use crate::durability::quest_state::npc_program::sandra_lottery::claim_key;
    let first = claim_key([1; 16], 1);
    assert_eq!(first, claim_key([1; 16], 1));
    assert_ne!(first, claim_key([1; 16], 2));
    assert_ne!(first, claim_key([2; 16], 1));
}

#[test]
fn sandra_lottery_source_planner_requires_owned_ticket() -> super::TestResult {
    use oteryn_gameplay_quests::quest::npc_rules::{
        NpcInventorySnapshot, NpcItemKey, NpcRuleCatalog, NpcRuleInput, NpcRuleTrigger,
    };
    use std::collections::BTreeMap;
    let source = NpcRuleCatalog::from_json_documents(&[include_str!(
        "../../../../../../content/dialogues/quest-rules-supplemental/sandra-lottery.json"
    )])?;
    let tracks = BTreeMap::from([(
        "oteryn:quest-progress/quest/u7_8/mage_and_summoner_outfits/addon_belt".into(),
        1,
    )]);
    let outfits = BTreeMap::new();
    for count in [0, 1] {
        let mut inventory = NpcInventorySnapshot::default();
        if count > 0 {
            inventory.items.insert(
                NpcItemKey {
                    item: SANDRA_LOTTERY.input.into(),
                    subtype: None,
                },
                count,
            );
        }
        let before = inventory.clone();
        let result = source.plan_item_exchange(&NpcRuleInput {
            npc: "oteryn:npc.sandra",
            trigger: NpcRuleTrigger::Say,
            message: "lottery",
            player_name: None,
            topic: 0,
            tracks: &tracks,
            inventory: &inventory,
            now_epoch_seconds: 0,
            calendar_month: 1,
            premium_current: false,
            level: 1,
            vocation: "oteryn:vocation.none",
            sex: 0,
            outfits: &outfits,
        })?;
        assert_eq!(result.is_some(), count == 1);
        if let Some(plan) = result {
            assert_eq!(plan.selected_rule_keys, ["oteryn:quest-npc/sandra/lottery"]);
        }
        assert_eq!(inventory, before, "planning must never mutate custody");
    }
    Ok(())
}
