//! D186: real fenced melee/cleave commits, species-bound critical charms.
use super::party_xp_fixtures::{self as party_xp, Case};
use crate::bestiary_postgres_harness::TestResult;


#[test]
fn all_charms_secondary_postgres_critical_commits_shared_roll_and_species() -> TestResult {
    party_xp::run(Case::CharmSecondary)
}
