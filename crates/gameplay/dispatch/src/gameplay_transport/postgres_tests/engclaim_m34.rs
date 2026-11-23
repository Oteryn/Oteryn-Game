//! ENGCLAIM: native authored server content through the production dispatch owner and PostgreSQL.
use super::TestResult;
use super::engclaim_m34_fixtures::execute;

#[test]
fn engclaim_m34_kormarak_production_multi_reward_once() -> TestResult {
    execute("kormarak-corpse")
}
#[test]
fn engclaim_m34_magic_sword_production_chosen_vault_once() -> TestResult {
    execute("magic_sword-north")
}

#[test]
fn engclaim_m34_mastermind_personal_cache_current_premium_once() -> TestResult {
    execute("mastermind_potion-wall")
}
