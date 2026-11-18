//! Lion's Rock compatibility through the native pedestal USE path.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn tears_uses_global_trial_feedback() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::m07_lion_offerings::run(
        "m07/lion-tears",
        Position {
            x: 33154,
            y: 32279,
            floor: -8,
        },
    )
}
