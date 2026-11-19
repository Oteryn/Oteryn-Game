//! S13c: production party conditions reach only nearby members, including the caster.
use super::party_buffs_fixtures::{run_party_buffs};

use crate::bestiary_postgres_harness::TestResult;

#[test]
fn party_buffs_real_dispatch_applies_values_and_duration_only_in_range_postgres() -> TestResult {
    run_party_buffs(None)
}

#[test]
fn heal_party_real_dispatch_members_values_duration_postgres() -> TestResult {
    run_party_buffs(Some(("heal_party", "druid")))
}
#[test]
fn enchant_party_real_dispatch_members_values_duration_postgres() -> TestResult {
    run_party_buffs(Some(("enchant_party", "sorcerer")))
}
#[test]
fn train_party_real_dispatch_members_values_duration_postgres() -> TestResult {
    run_party_buffs(Some(("train_party", "knight")))
}
