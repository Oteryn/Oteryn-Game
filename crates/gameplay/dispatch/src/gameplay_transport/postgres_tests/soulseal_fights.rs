//! T114: actual admitted command frames reach the seal debit and real Instance handoff.
use super::soulseal_fights_fixtures::{Mode};

use super::TestResult;

#[test]
fn t114_soulseal_real_dispatch_spends_once_and_admits_solo() -> TestResult {
    run(Mode::Spend)
}
#[test]
fn t114_soulseal_refused_capacity_preserves_seals() -> TestResult {
    run(Mode::Capacity)
}
#[test]
fn t114_soulseal_failed_successor_rolls_back_payment() -> TestResult {
    run(Mode::Rollback)
}
#[test]
fn t114_soulseal_insufficient_balance_cannot_create_encounter() -> TestResult {
    run(Mode::Insufficient)
}
fn run(mode: Mode) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::soulpit_shared_entry_fixtures::native::soulseal_scenario(mode)
}
