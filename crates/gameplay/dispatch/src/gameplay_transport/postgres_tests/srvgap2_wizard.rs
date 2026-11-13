//! Both retained Wizard addon sources through native NPC commands and PostgreSQL.
#[test]
fn srvgap2_wizard_source_story() -> super::TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l2-wizard-skull-story")
}
