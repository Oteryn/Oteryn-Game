//! Real boss settlement -> personal browse/take/collect-all dispatch -> guarded Item custody.
use super::boss_rewards_fixtures::{Case, run_case};

use super::TestResult;


#[test]
fn boss_rewards_postgres_dispatch_collection_replay_capacity_and_custody_guards() -> TestResult {
    run_case(Case::Collection)
}

#[test]
fn boss_rewards_postgres_dispatch_seven_day_expiry_private_scope_and_stale_fences() -> TestResult {
    run_case(Case::Expiry)
}

#[test]
fn boss_rewards_postgres_native_lethal_settlement_and_later_replay() -> TestResult {
    run_case(Case::NativeDeath)
}

#[test]
fn boss_rewards_postgres_production_boss_lethal_to_personal_item_custody() -> TestResult {
    run_case(Case::ProductionNativeDeath)
}

#[test]
fn boss_rewards_postgres_disconnected_contributor_keeps_earned_entitlement() -> TestResult {
    run_case(Case::OfflineReward)
}

#[test]
fn boss_rewards_postgres_live_rewards_are_private_between_current_characters() -> TestResult {
    run_case(Case::LivePrivacy)
}

#[test]
fn boss_rewards_postgres_concurrent_collectors_on_two_roots_transfer_once() -> TestResult {
    run_case(Case::ConcurrentCollection)
}
