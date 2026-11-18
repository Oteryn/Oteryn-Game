//! Lion's Rock compatibility through the native pedestal USE path.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn tears_uses_global_trial_feedback() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qcx-lions-rock-tears", || {
        super::quest_world_callbacks::scenario(
            "m07/lion-tears",
            Position {
                x: 33154,
                y: 32279,
                floor: -8,
            },
            false,
        )
    });
    eprintln!("QCX Lion's Rock Tears elapsed {:?}", started.elapsed());
    result
}
