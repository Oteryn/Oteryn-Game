//! PARITY-EQUIP-MODIFIERS-1: equipment critical hit and leech on real auto-attack dispatch.
use super::party_xp_fixtures::{self as party_xp, Case};
use crate::bestiary_postgres_harness::TestResult;

#[test]
fn equipped_offence_postgres_real_swings_critical_hit_and_leech() -> TestResult {
    party_xp::run(Case::Offence)
}
