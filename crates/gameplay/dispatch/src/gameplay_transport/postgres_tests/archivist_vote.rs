//! Crystal 201189f8 data-global/npc/marvin.lua:57-85.

use super::TestResult;
use super::archivist_vote_fixtures::vote_case;

#[test]
fn archivist_confirmation_debits_oramond_and_credits_dark_trails_in_one_revision() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    vote_case(admin, "archivist_vote", 2, 1)
}

#[test]
fn archivist_confirmation_credits_dark_trails_after_votes_are_exhausted() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    vote_case(admin.clone(), "archivist_vote_exhausted", 0, 0)?;
    vote_case(admin, "archivist_vote_unset", -1, -1)
}
