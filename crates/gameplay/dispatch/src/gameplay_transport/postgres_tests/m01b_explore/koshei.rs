use super::*;

#[test]
fn m01b_koshei_four_native_visits_require_level_and_cannot_be_farmed() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-koshei-fragments", || {
            super::super::quest_world_callbacks_fixtures::scenario(
                "m01b/koshei-the-deathless/s1/koshei_amulet_part1",
                Position {
                    x: 33195,
                    y: 32458,
                    floor: -7,
                },
                false,
            )
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_koshei_fragments duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
