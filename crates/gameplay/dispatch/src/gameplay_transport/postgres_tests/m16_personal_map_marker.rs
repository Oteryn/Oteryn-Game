//! M16: Sterling confirmation -> fenced receipt -> domain 16 delta and reconnect snapshot.

use super::TestResult;


#[test]
fn m16_sterling_commits_personal_marker_and_delivers_delta_and_reconnect_snapshot() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("m16-marker", || {
        super::quest_world_callbacks_fixtures::scenario(
            "m16-marker",
            oteryn_gameplay_quests::quest::world_callbacks::Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })
}
