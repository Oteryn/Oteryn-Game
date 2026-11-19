//! W19b: Crystal dryad_garden.lua:7-24 teleports before emitting watersplash.
use super::TestResult;

use super::quest_world_callbacks_fixtures::scenario;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn srvwld_dryad_garden_enter_teleports_and_publishes_destination_splash() -> TestResult {
    garden(
        "dryad-garden-enter",
        Position {
            x: 33264,
            y: 32012,
            floor: -7,
        },
    )
}

#[test]
fn srvwld_dryad_garden_leave_teleports_and_publishes_destination_splash() -> TestResult {
    garden(
        "dryad-garden-leave",
        Position {
            x: 33202,
            y: 32012,
            floor: -11,
        },
    )
}

fn garden(name: &'static str, trigger: Position) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || scenario(name, trigger, false))?;
    eprintln!(
        "SRVWLD_DRYAD_POSTGRES_CASE_COMPLETED {name} elapsed_seconds={:.3}",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
