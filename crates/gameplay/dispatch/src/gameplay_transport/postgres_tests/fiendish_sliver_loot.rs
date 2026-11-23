//! A real Fiendish kill inserts a single sliver stack regardless of scheduled loot rates.
use super::TestResult;
use super::party_xp_fixtures::{self as party_xp, Case};

#[test]
fn fiendish_sliver_low_loot_rate_postgres() -> TestResult {
    party_xp::run(Case::ForgeSliverLowLootRate)
}

#[test]
fn fiendish_sliver_double_loot_postgres() -> TestResult {
    party_xp::run(Case::ForgeSliverDoubleLoot)
}
