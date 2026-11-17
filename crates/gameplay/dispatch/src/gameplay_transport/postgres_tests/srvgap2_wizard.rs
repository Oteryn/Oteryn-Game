//! Both retained Wizard addon sources through native NPC commands and PostgreSQL.
#[cfg(feature = "pg-tests-g1")]
#[test]
fn srvgap2_wizard_source_story() -> super::TestResult {
    super::qengine_rewards::scenario("qengine-rewards-l2-wizard-skull-story")
}
