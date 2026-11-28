//! Exact M25 boss predecessors use fenced fixture setup; progress is actual native lethal work.

use super::TestResult;

#[test]
fn m25_galthen_real_native_deaths_require_predecessors_and_never_complete_postgres() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-m25-galthen-native-deaths")
}

#[test]
fn m25_galthen_boss_award_survives_scope_replacement_and_keeps_earning_account() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-m25-galthen-native-deaths-takeover")
}

#[test]
fn m25_galthen_boss_award_survives_xp_failure_postgres() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-m25-galthen-native-deaths-xp")
}

#[test]
fn m25_galthen_boss_award_survives_departed_top_damager_postgres() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-m25-galthen-native-deaths-departed")
}

#[test]
fn m25_galthen_boss_award_recovers_on_idle_successor_postgres() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-m25-galthen-native-deaths-idle")
}
