//! A lost Ice Islands paint acknowledgement replays its exact selected-tool binding.
use super::engitem_seal_paint_lost_ack_fixtures::{NAME};

use super::engitem_seal_paint_fixtures::{SEALS, position};

use super::TestResult;

#[test]
fn lost_ack_ice_islands_painting_replay_keeps_selected_tool_binding() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::kill_reward_e2e_fixtures::scenario_thread::run("engitem_seal_paint_lost_ack", || {
            super::quest_world_callbacks_fixtures::scenario(NAME, position(SEALS[0])?, false)
        });
    eprintln!(
        "ENGITEM_PG_EXECUTED seal_paint_lost_ack duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
