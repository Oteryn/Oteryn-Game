//! Albinius's five-tome contribution through the ordinary NPC/item/quest owners.

use super::TestResult;

#[test]
fn questpc2_albinius_four_tomes_and_another_book_cannot_pay_the_contribution() -> TestResult {
    super::qengine_rewards_fixtures::scenario("questpc2-albinius-four-tomes")
}

#[test]
fn questpc2_albinius_five_tomes_commit_with_progress_and_preserve_other_books() -> TestResult {
    super::qengine_rewards_fixtures::scenario("questpc2-albinius-five-tomes")
}

#[test]
fn questpc2_albinius_combined_offer_and_confirmation_preserve_source_if_order() -> TestResult {
    super::qengine_rewards_fixtures::scenario("questpc2-albinius-combined")
}
