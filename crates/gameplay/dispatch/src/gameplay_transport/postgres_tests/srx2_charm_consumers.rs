//! SRX-2: real auto-attack damage consumes a killing-hit charm without chaining.
use super::party_xp_fixtures::{self as party_xp, Case};
use crate::bestiary_postgres_harness::TestResult;

#[test]
fn srx2_charm_carnage_real_lethal_swing_and_adjacent_damage() -> TestResult {
    party_xp::run(Case::CharmCarnage)
}

#[test]
fn srx2_charm_incoming_hit_hastes_player() -> TestResult {
    party_xp::run(Case::CharmIncoming)
}

#[test]
fn srx2_charm_numb_incoming_hit_paralyses_attacker() -> TestResult {
    party_xp::run(Case::CharmNumb)
}

#[test]
fn srx2_charm_cleanse_incoming_hit_removes_one_negative_condition() -> TestResult {
    party_xp::run(Case::CharmCleanse)
}

#[test]
fn srx2_charm_cripple_real_swing_slows_living_target() -> TestResult {
    party_xp::run(Case::CharmCripple)
}

#[test]
fn srx2_charm_fatal_hold_real_swing_prevents_native_flee() -> TestResult {
    party_xp::run(Case::CharmFatalHold)
}

#[test]
fn srx2_charm_void_inversion_native_source_mana_commit_and_replay() -> TestResult {
    party_xp::run(Case::CharmMana)
}

#[test]
fn srx2_charm_bless_lethal_snapshot_atomic_loss_rollback_and_replay() -> TestResult {
    party_xp::run(Case::CharmBless)
}

#[test]
fn srx2_charm_parry_native_source_spell_damage_and_replay() -> TestResult {
    party_xp::run(Case::CharmParry)
}
