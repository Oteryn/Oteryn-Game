use super::*;

#[test]
fn imbuement_reference_parity() {
    // One row per task-card parity row; sources live beside the constants.
    enum Rule {
        Tier(ImbuementTier, u64, bool, u8),
        Duration(u64),
        Slots(u8),
        Clear(u64),
        Checkpoint(u64),
    }
    let rows = [
        Rule::Tier(ImbuementTier::Basic, 5_000, false, 100),
        Rule::Tier(ImbuementTier::Intricate, 30_000, true, 100),
        Rule::Tier(ImbuementTier::Powerful, 200_000, true, 100),
        Rule::Duration(20 * 60 * 60 * 1_000),
        Rule::Slots(3),
        Rule::Clear(15_000),
        Rule::Checkpoint(60 * 1_000),
    ];
    for row in rows {
        match row {
            Rule::Tier(tier, apply_gold, requires_premium, success_percent) => {
                assert_eq!(
                    IMBUEMENT_TIER_RULES.iter().find(|rules| rules.tier == tier),
                    Some(&ImbuementTierRules {
                        tier,
                        apply_gold,
                        requires_premium,
                        success_percent,
                    }),
                );
            }
            Rule::Duration(value) => assert_eq!(IMBUEMENT_DURATION_MS, value),
            Rule::Slots(value) => assert_eq!(IMBUEMENT_SLOTS_MAX, value),
            Rule::Clear(value) => assert_eq!(IMBUEMENT_CLEAR_GOLD, value),
            Rule::Checkpoint(value) => assert_eq!(IMBUEMENT_CHECKPOINT_INTERVAL_MS, value),
        }
    }
}

#[test]
fn imbuement_catalogue_matches_tier_rules_and_covers_every_single_family() {
    let mut expected = Vec::new();
    for family in (1..=20).filter(|family| *family != 2) {
        for tier in 1..=3 {
            expected.push((family, tier));
        }
    }
    assert_eq!(
        IMBUEMENT_RECIPES
            .iter()
            .map(|recipe| (recipe.family, recipe.tier))
            .collect::<Vec<_>>(),
        expected
    );
    for recipe in &IMBUEMENT_RECIPES {
        let rules = &IMBUEMENT_TIER_RULES[usize::from(recipe.tier) - 1];
        assert_eq!(rules.tier as u8, recipe.tier);
        assert_eq!(rules.apply_gold, recipe.apply_gold);
        assert_eq!(rules.requires_premium, recipe.requires_premium);
        assert_eq!(
            recipe.materials.len(),
            usize::from(recipe.tier),
            "{recipe:?}"
        );
        assert!(recipe.key.starts_with("oteryn:imbuement."));
        assert!(recipe.materials.iter().all(|(key, count)| {
            key.starts_with("oteryn:item.tibia.i") && (1..=100).contains(count)
        }));
        assert_eq!(imbuement_recipe(recipe.family, recipe.tier), Some(recipe));
    }
    assert_eq!(imbuement_recipe(2, 1), None);
    assert_eq!(IMBUEMENT_CATALOGUE_REVISION.len(), 64);
}

#[test]
fn filled_scroll_ids_round_trip_only_intricate_and_powerful_recipes() {
    let mut ids = std::collections::BTreeSet::new();
    for scroll in IMBUEMENT_SCROLLS {
        assert!(ids.insert(scroll.item_id), "duplicate scroll id");
        assert!(matches!(scroll.tier, 2 | 3));
        assert!(imbuement_recipe(scroll.family, scroll.tier).is_some());
        assert_eq!(imbuement_scroll(scroll.item_id), Some(scroll));
        assert_eq!(
            scroll_for_imbuement(scroll.family, scroll.tier),
            Some(scroll)
        );
    }
    assert_eq!(ids.len(), 36);
    assert_eq!(imbuement_scroll(IMBUEMENT_ETCHER_ITEM_ID), None);
    assert_eq!(imbuement_scroll(51_443), None);
    assert_eq!(scroll_for_imbuement(1, 1), None);
    assert_eq!(scroll_for_imbuement(13, 2), None);
    assert_eq!(BLANK_IMBUEMENT_SCROLL_GOLD, 25_000);
    assert_eq!(BLANK_IMBUEMENT_SCROLL_TOMES, 5);
    assert_eq!(BLANK_IMBUEMENT_SCROLL_ITEM_ID, 51_442);
    assert!(can_buy_blank_imbuement_scroll(true, 5));
    assert!(!can_buy_blank_imbuement_scroll(false, 5));
    assert!(!can_buy_blank_imbuement_scroll(true, 4));
    assert_eq!(
        qualify_scroll_inscription(1, 3, true, true)
            .map(|(scroll, recipe)| (scroll.item_id, recipe.key)),
        Some((51_462, "oteryn:imbuement.strike"))
    );
    assert!(qualify_scroll_inscription(1, 1, true, true).is_none());
    assert!(qualify_scroll_inscription(1, 3, false, true).is_none());
    assert!(qualify_scroll_inscription(1, 3, true, false).is_none());
}
