//! Para Norbert's confirmation from TibiaWiki curid 89657, oldid 1153851.
use super::{TestResult, kill_reward_e2e_fixtures::scenario_thread, quest_world_callbacks_fixtures as quest_world_callbacks};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn norbert_yes_grants_one_death_knell_to_a_premium_player() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }

    let started = std::time::Instant::now();
    let result = scenario_thread::run("qcx-feaster-norbert", || {
        quest_world_callbacks::scenario(
            "qengine-rewards-feaster-prefix",
            Position {
                x: 32627,
                y: 32076,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "QCX_FEASTER_NORBERT POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
