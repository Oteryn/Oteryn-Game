//! M39: both sample outcomes use native commands, carried custody and fenced XP.
//! Reuse L2 1819e54c7; Muriel9f5a72c npc/muriel.lua:61-96,130-139,
//! sha256:7bd9180ba903509cb3a4a6fe01a7b73d9038eff99d15c68bf95fed5ec697d4cc.

use super::TestResult;

#[test]
fn m39_bone_pit_full_player_journey() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-m39-bone")
}
#[test]
fn m39_bone_pit_lost_sample_journey() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-m39-bone-lost")
}
