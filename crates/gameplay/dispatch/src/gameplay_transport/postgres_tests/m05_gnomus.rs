//! M05: the production NPC caller commits the donor Gnomus reward atomically.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;


#[test]
fn m05_gnomus_dispatch_commits_item_points_deadline_and_bound_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("m05-gnomus", || {
        super::quest_world_callbacks_fixtures::scenario(
            "m05-gnomus",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!("M05_DISPATCH_PG_EXECUTED item_points_deadline_replay");
    Ok(())
}
