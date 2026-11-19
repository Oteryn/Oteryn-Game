//! The two historical bridge routes share native scene and carried-tool owners.
use super::{TestResult, quest_world_callbacks_fixtures as quest_world_callbacks};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

fn scenario(name: &'static str, trigger: Position) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("questpc8-bridge-route", move || {
        quest_world_callbacks::scenario(name, trigger, false)
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_bridge_route {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn questpc8_life_ring_native_bridge_pick_cache_and_exact_rewards() -> TestResult {
    // Reuse the committed programme and bridge scenarios after shared-engine convergence.
    scenario(
        "authored-life-ring-entry",
        Position {
            x: 32437,
            y: 32238,
            floor: -10,
        },
    )?;
    scenario(
        "authored-life-ring-bridge",
        Position {
            x: 32414,
            y: 32231,
            floor: -10,
        },
    )
}

#[test]
fn questpc8_torch_native_academy_lever_rat_and_reward_once() -> TestResult {
    scenario(
        "authored-torch",
        Position {
            x: 32097,
            y: 32204,
            floor: -8,
        },
    )
}
