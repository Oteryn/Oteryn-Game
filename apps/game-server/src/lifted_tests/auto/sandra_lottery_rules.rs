//! Lottery rule assertions shared by the two directory-mounted test targets.
use oteryn_gameplay_quests::quest::npc_rules::NpcRuleEffect;
use oteryn_gameplay_quests::quest::npc_rules::random_outcome::SANDRA_LOTTERY;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn sandra_lottery_accepts_previous_outfit_definition_pin() -> TestResult {
    use oteryn_gameplay_quests::quest::{QuestStateCatalogue, loader, npc_rules};
    let loaded =
        loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
    let current = loaded.catalogue();
    let source = npc_rules::NpcRuleCatalog::runtime()?;
    let sandra = source
        .programs()
        .iter()
        .find(|program| {
            program.document.program_key.as_deref()
                == Some("oteryn:quest-npc-program.sandra-lottery")
        })
        .ok_or("Sandra programme")?;
    let digest: String = sandra
        .digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let marker = format!("oteryn:quest-npc-pin/{digest}");
    assert!(current.transition(&marker).is_some());
    let previous = QuestStateCatalogue::new(
        current.content_revision(),
        current.declared_tracks().cloned().collect(),
        current
            .declared_transitions()
            .filter(|transition| transition.key != marker)
            .cloned()
            .collect(),
    )?;
    let quest = "oteryn:quest.outfit_and_addon_quests";
    let mut hash = previous
        .definition_hash(quest)
        .ok_or("previous outfit definition")?;
    assert_ne!(Some(hash), current.definition_hash(quest));
    assert!(
        npc_rules::legacy_defaults(current, quest, &hash)
            .is_some_and(std::collections::BTreeMap::is_empty),
        "the exact pre-Sandra definition must preserve its existing defaults"
    );
    hash[0] ^= 1;
    assert!(npc_rules::legacy_defaults(current, quest, &hash).is_none());
    Ok(())
}

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
fn sandra_lottery_dialogue_is_registered_for_production() -> TestResult {
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
fn sandra_lottery_source_planner_requires_owned_ticket() -> TestResult {
    use oteryn_gameplay_quests::quest::npc_rules::{
        NpcInventorySnapshot, NpcItemKey, NpcRuleCatalog, NpcRuleInput, NpcRuleTrigger,
    };
    use std::collections::BTreeMap;
    let source = NpcRuleCatalog::runtime()?;
    let tracks = BTreeMap::from([(
        "oteryn:quest-progress/quest/u7_8/mage_and_summoner_outfits/addon_belt".into(),
        2,
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

#[test]
fn sandra_lottery_selected_inventory_matches_win_and_blank() {
    use oteryn_gameplay_quests::quest::npc_rules::{NpcInventorySnapshot, NpcItemKey};
    let input = NpcItemKey {
        item: SANDRA_LOTTERY.input.into(),
        subtype: None,
    };
    let prize = NpcItemKey {
        item: "oteryn:item.tibia.i5958".into(),
        subtype: None,
    };
    let mut before = NpcInventorySnapshot::default();
    assert!(SANDRA_LOTTERY.inventory_after(&before, 0).is_none());
    before.items.insert(input.clone(), 2);
    before.items.insert(prize.clone(), 3);
    let mut expected = before.clone();
    expected.items.insert(input, 1);
    assert_eq!(
        SANDRA_LOTTERY.inventory_after(&before, 97),
        Some(expected.clone())
    );
    expected.items.insert(prize, 4);
    assert_eq!(SANDRA_LOTTERY.inventory_after(&before, 0), Some(expected));
    assert_eq!(
        before.items.get(&NpcItemKey {
            item: SANDRA_LOTTERY.input.into(),
            subtype: None
        }),
        Some(&2)
    );
}

#[test]
fn sandra_lottery_rejects_extra_or_optional_inventory_effects() -> TestResult {
    use crate::durability::quest_state::npc_program::sandra_lottery::valid_ticket_effects;
    use oteryn_gameplay_quests::quest::npc_rules::NpcRuleEffect;
    let mut effects = SANDRA_LOTTERY.effects(1, |_| 0).ok_or("winning effects")?;
    assert!(valid_ticket_effects(&effects));
    effects.push(effects[0].clone());
    assert!(!valid_ticket_effects(&effects));
    effects.pop();
    if let NpcRuleEffect::TakeItem { optional, .. } = &mut effects[0] {
        *optional = true;
    }
    assert!(!valid_ticket_effects(&effects));
    assert!(!valid_ticket_effects(&[]));
    Ok(())
}

#[test]
fn sandra_lottery_refuses_unrelated_rewards_before_inventory_mutation() -> TestResult {
    use crate::durability::quest_state::npc_program::sandra_lottery::valid_ticket_effects;
    use oteryn_gameplay_quests::quest::npc_rules::NpcRuleEffect;
    let base = SANDRA_LOTTERY.effects(1, |_| 0).ok_or("winning effects")?;
    for effect in [
        NpcRuleEffect::Experience { amount: 1 },
        NpcRuleEffect::GiveMoney {
            amount: 1,
            include_bank: false,
        },
        NpcRuleEffect::OutfitAddon {
            look_type: 138,
            addon: 1,
        },
    ] {
        let mut effects = base.clone();
        effects.push(effect);
        assert!(!valid_ticket_effects(&effects));
    }
    assert!(valid_ticket_effects(&base));
    Ok(())
}

#[test]
fn sandra_lottery_ticket_issuance_matches_donor_same_kind_vials() -> TestResult {
    use oteryn_gameplay_quests::quest::npc_rules::{NpcRuleCatalog, NpcRuleEffect, NpcRuleGuard};
    let source = NpcRuleCatalog::runtime()?;
    let program = source
        .programs()
        .iter()
        .find(|program| {
            program.document.program_key.as_deref()
                == Some("oteryn:quest-npc-program.sandra-lottery")
        })
        .ok_or("Sandra source programme")?;
    let rules = &program.document.rules;
    for item in [283, 284, 285] {
        let key = format!("oteryn:item.tibia.i{item}");
        let rule = rules
            .iter()
            .find(|rule| rule.key == format!("oteryn:quest-npc/sandra/deposit-{item}"))
            .ok_or("donor vial exchange")?;
        assert!(rule.guards.iter().any(|guard| matches!(guard,
            NpcRuleGuard::Item { item, count: 100, .. } if item == &key)));
        assert!(rule.effects.iter().any(|effect| matches!(effect,
            NpcRuleEffect::TakeItem { item, count: 100, optional: false, .. } if item == &key)));
        assert!(rule.effects.iter().any(|effect| matches!(effect,
            NpcRuleEffect::GiveItem { item, count: 1, .. } if item == SANDRA_LOTTERY.input)));
    }
    assert!(source.item_exchange_keys("oteryn:npc.sandra").is_some());
    Ok(())
}

#[test]
fn sandra_lottery_vial_deposits_use_repeatable_command_claims() {
    use crate::durability::quest_state::npc_program::sandra_lottery::repeatable_exchange;
    for suffix in ["lottery", "deposit-283", "deposit-284", "deposit-285"] {
        assert!(repeatable_exchange(&format!(
            "oteryn:quest-npc/sandra/{suffix}"
        )));
    }
    for suffix in ["bonus-join", "prize", "deposit-286", "deposit-283-extra"] {
        assert!(!repeatable_exchange(&format!(
            "oteryn:quest-npc/sandra/{suffix}"
        )));
    }
    assert!(!repeatable_exchange("oteryn:quest-npc/other/lottery"));
}

#[test]
fn sandra_lottery_bonus_topics_match_donor_enrollment_flow() -> TestResult {
    use oteryn_gameplay_quests::quest::npc_rules::{
        NpcInventorySnapshot, NpcRuleCatalog, NpcRuleInput, NpcRuleTrigger,
    };
    use std::collections::BTreeMap;
    let source = NpcRuleCatalog::runtime()?;
    let inventory = NpcInventorySnapshot::default();
    let outfits = BTreeMap::new();
    let track = "oteryn:quest-progress/quest/u7_8/mage_and_summoner_outfits/addon_belt";
    for (value, message, topic, expected) in [
        (0, "ticket", 0, 1),
        (0, "yes", 1, 2),
        (0, "yes", 2, 0),
        (1, "deposit", 0, 3),
        (2, "bonus", 0, 3),
    ] {
        let tracks = BTreeMap::from([(track.into(), value)]);
        let plan = source
            .plan_item_exchange(&NpcRuleInput {
                npc: "oteryn:npc.sandra",
                trigger: NpcRuleTrigger::Say,
                message,
                player_name: None,
                topic,
                tracks: &tracks,
                inventory: &inventory,
                now_epoch_seconds: 0,
                calendar_month: 1,
                premium_current: false,
                level: 1,
                vocation: "oteryn:vocation.none",
                sex: 0,
                outfits: &outfits,
            })?
            .ok_or("bonus dialogue")?;
        assert_eq!(plan.next_topic, expected);
        if topic == 2 {
            assert!(plan.track_changes.iter().any(|change| change.key
                == "oteryn:quest-progress/outfit_quest/default_start"
                && change.after == 1));
            assert!(
                plan.track_changes
                    .iter()
                    .any(|change| change.key == track && change.after == 1)
            );
        }
    }
    Ok(())
}

#[test]
fn sandra_lottery_donor_entry_keywords_match_entire_message() -> TestResult {
    use oteryn_gameplay_quests::quest::npc_rules::{
        NpcInventorySnapshot, NpcRuleCatalog, NpcRuleInput, NpcRuleTrigger,
    };
    use std::collections::BTreeMap;
    let source = NpcRuleCatalog::runtime()?;
    let inventory = NpcInventorySnapshot::default();
    let outfits = BTreeMap::new();
    for value in [0, 1] {
        let tracks = BTreeMap::from([(
            "oteryn:quest-progress/quest/u7_8/mage_and_summoner_outfits/addon_belt".into(),
            value,
        )]);
        for message in ["a vial", "ticket please", "bonus system", "deposit vials"] {
            let plan = source.plan_item_exchange(&NpcRuleInput {
                npc: "oteryn:npc.sandra",
                trigger: NpcRuleTrigger::Say,
                message,
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
            assert!(plan.is_none(), "donor requires an exact entry keyword");
        }
    }
    Ok(())
}
