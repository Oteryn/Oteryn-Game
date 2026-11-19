//! M39a: twelve physical source Tomes, incremental NPC topics and exact hand-ins.
//! Crystal9f5a72c cael.lua:84-133 sha256:4d4d0c4aa36722ebb85925c685b7631471889dd1f51e641711cee51568336dcd.

use super::TestResult;


#[test]
fn m39a_tomes_twelve_source_books_and_incremental_handins() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-m39-tomes")
}

#[test]
fn m39b_tomes_repeatable_paid_xp_and_no_free_reward() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-m39-tomes-extra")
}

#[test]
fn m39_tomes_legacy_claim_blocks_a_second_world_reward() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-m39-tomes-legacy-first")
}

#[test]
fn m39_tomes_world_claim_blocks_a_second_legacy_reward() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-m39-tomes-world-first")
}
