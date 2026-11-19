//! Order documents must enter through their NPC source and ordinary carried USE.

use super::TestResult;

#[test]
fn qengine_l10_brotherhood_complete_document_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l10-documents-brotherhood-story")
}
#[test]
fn qengine_l10_nightmare_complete_document_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l10-documents-nightmare-story")
}
