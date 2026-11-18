//! Spike Tasks return travel through the real STEP dispatch path.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn spike_return_portal_reaches_kazordoon_without_quest_mutation() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qcx-spike-return", || {
        super::quest_world_callbacks::stateless_scenario(
            "spike-portal-4227",
            Position {
                x: 32242,
                y: 32597,
                floor: -8,
            },
            Position {
                x: 32624,
                y: 31855,
                floor: -11,
            },
        )
    });
    eprintln!("spike_return_portal elapsed: {:.2?}", started.elapsed());
    result
}
