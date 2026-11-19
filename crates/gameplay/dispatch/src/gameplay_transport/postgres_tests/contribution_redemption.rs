//! Crystal 201189f8 data-global/npc/sandomo.lua:53-59,97-165.

use super::TestResult;
use super::contribution_redemption_fixtures::{exercise, exercise_bulk, redemption_case};

#[test]
fn contribution_books_reset_counts_and_two_quantity_choices_redeem_exact_gold() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    redemption_case(admin, "contribution_redemption", 500, exercise)
}

#[test]
fn contribution_redemption_mints_two_stacks_and_replay_does_not_charge_again() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    redemption_case(admin, "contribution_bulk_redemption", 10_103, exercise_bulk)
}
