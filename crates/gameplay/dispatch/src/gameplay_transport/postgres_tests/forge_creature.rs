//! FORGECREATURE: existing real attacks, multi-recipient settlement, replay and capped dust.
use super::party_xp_fixtures::{self as party_xp, Case};
use crate::bestiary_postgres_harness::TestResult;

#[test]
fn forge_creature_postgres_real_dispatch_party_cap_and_replay() -> TestResult {
    party_xp::run(Case::ForgeReplay)
}
#[test]
fn forge_creature_postgres_real_dispatch_premium_gate_and_replay() -> TestResult {
    party_xp::run(Case::ForgeFree)
}
#[test]
fn forge_creature_postgres_real_dispatch_idle_party_member_gets_no_dust() -> TestResult {
    party_xp::run(Case::ForgeIdle)
}
#[test]
fn forge_creature_postgres_real_dispatch_free_players_still_loot_slivers() -> TestResult {
    party_xp::run(Case::ForgeAllFree)
}
#[test]
fn forge_creature_postgres_real_dispatch_shared_xp_disabled_still_awards_dust() -> TestResult {
    party_xp::run(Case::ForgeNoShared)
}
#[test]
fn forge_creature_postgres_real_dispatch_stale_lease_awards_nothing() -> TestResult {
    party_xp::run(Case::ForgeStale)
}
#[test]
fn forge_creature_postgres_real_dispatch_member_failure_rolls_back_dust_and_xp() -> TestResult {
    party_xp::run(Case::ForgeRollback)
}
#[test]
fn forge_creature_postgres_real_dispatch_party_range_excludes_distant_member() -> TestResult {
    party_xp::run(Case::ForgeFar)
}
#[test]
fn forge_creature_postgres_real_dispatch_foreign_contributor_gets_full_dust() -> TestResult {
    party_xp::run(Case::ForgeForeign)
}

#[test]
fn fiendish_slivers_postgres_survive_reduced_loot() -> TestResult {
    party_xp::run(Case::ForgeReducedLoot)
}

#[test]
fn fiendish_slivers_postgres_are_awarded_once_during_double_loot() -> TestResult {
    party_xp::run(Case::ForgeDoubleLoot)
}

#[test]
fn fiendish_sliver_postgres_keeps_one_cause_when_duplex_expires_before_task_board_retry() -> TestResult {
    party_xp::run(Case::ForgeDuplexRetry)
}

#[test]
fn fiendish_slivers_postgres_survive_boosted_rat_at_exhausted_stamina() -> TestResult {
    party_xp::run(Case::ForgeBoostedExhausted)
}

#[test]
fn fiendish_slivers_postgres_survive_boosted_rat_at_exhausted_stamina_during_double_loot()
-> TestResult {
    party_xp::run(Case::ForgeBoostedExhaustedDoubleLoot)
}

#[test]
fn elite_dust_real_party_shared_off_excludes_non_damaging_member() -> TestResult {
    party_xp::run(Case::EliteNoShared)
}
