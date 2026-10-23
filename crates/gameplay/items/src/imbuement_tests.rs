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
