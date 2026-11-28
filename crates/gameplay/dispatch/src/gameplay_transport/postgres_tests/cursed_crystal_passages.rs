//! Crystal's four fixed passage landings run through the ordinary STEP owner.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn cursed_crystal_stairs_step_to_all_four_donor_landings() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("cursed-crystal-stairs", || {
        let position = |(x, y, floor)| Position { x, y, floor };
        for (key, entry, exit) in [
            ("crystal-stairs-29", (31973, 32905, -11), (31971, 32904, -10)),
            ("crystal-stairs-31", (31973, 32905, -10), (31975, 32911, -9)),
            ("crystal-stairs-33", (32009, 32928, -9), (32009, 32929, -10)),
            ("crystal-stairs-35", (32009, 32928, -10), (32009, 32929, -9)),
        ] {
            super::quest_world_callbacks_fixtures::stateless_scenario(
                key, position(entry), position(exit),
            )?;
        }
        Ok(())
    })
}
