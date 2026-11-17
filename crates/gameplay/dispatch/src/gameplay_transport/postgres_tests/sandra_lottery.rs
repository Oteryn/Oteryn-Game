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
