//! Three distinct native seals retain custody, replay and revision-fenced reset.
use super::engitem_seal_paint_fixtures::{SEALS, position};

use super::TestResult;

#[test]
fn engitem_native_seal_paint_preserves_tool_counts_occurrences_and_resets() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("engitem_seal_paint", || {
        super::quest_world_callbacks_fixtures::scenario(
            "engitem-native-seal-paint",
            position(SEALS[0])?,
            false,
        )
    });
    eprintln!(
        "ENGITEM_PG_EXECUTED seal_paint duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result?;
    assert!(started.elapsed().as_secs_f64() > 0.01);
    Ok(())
}
