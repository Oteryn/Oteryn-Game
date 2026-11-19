//! Hoist STEP transport checks both instruction flags on every trip.

use super::{TestResult, quest_world_callbacks_fixtures as quest_world_callbacks};

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc7_formorgar_hoist_read_only_step() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-hoist", || {
        for (name, floor) in [
            ("formogar-hoist-3059", -9),
            ("formogar-hoist-3060", -10),
            ("formogar-hoist-3061", -11),
        ] {
            let start = std::time::Instant::now();
            quest_world_callbacks::scenario(
                name,
                Position {
                    x: 32157,
                    y: 31125,
                    floor,
                },
                false,
            )?;
            eprintln!(
                "QUESTPC7_HOIST POSTGRES_CASE_COMPLETED {name} duration={:?}",
                start.elapsed()
            );
        }
        Ok(())
    })?;
    Ok(())
}
