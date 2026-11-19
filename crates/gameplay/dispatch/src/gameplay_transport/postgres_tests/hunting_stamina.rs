//! Real auto-attack dispatch and the registered PostgreSQL durability path.
use super::party_xp_fixtures::{self as party_xp, Case};
use crate::bestiary_postgres_harness::TestResult;

#[path = "../../../../../../apps/game-server/tests/support/stamina_postgres_cases.rs"]
mod persistence;

#[test]
fn hunting_stamina_postgres_kill_experience_idle_checkpoint_and_reload() -> TestResult {
    party_xp::run(Case::Stamina)
}
