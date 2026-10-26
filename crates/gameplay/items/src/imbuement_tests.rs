use super::*;

#[test]
fn utility_categories_run_online_and_combat_categories_pause() {
    assert!(consumes_elapsed(8, false));
    assert!(consumes_elapsed(9, false));
    assert!(consumes_elapsed(0, true));
    assert!(!consumes_elapsed(0, false));
    assert!(!consumes_elapsed(19, false));
}

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
