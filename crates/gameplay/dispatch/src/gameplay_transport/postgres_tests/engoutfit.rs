//! Shared outfit owner proofs through the production NPC dispatcher and PostgreSQL.
use super::TestResult;

#[test]
fn engoutfit_battle_mage_atomic_outfit_and_addons() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l2-battle-mage-story")
}

#[test]
fn engoutfit_beggar_atomic_outfit_and_addons() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l6-beggar-female-story")
}

#[test]
fn engoutfit_assassin_atomic_outfit_and_addons() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l7-assassin-story")
}

