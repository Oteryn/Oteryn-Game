use super::*;

#[test]
fn m01b_katana_native_key_route_entry_credits_once_without_claiming_key() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-katana-key-entry", move || {
            super::super::quest_world_callbacks_fixtures::scenario(
                "m01b/katana/s1/id4603",
                Position {
                    x: 32177,
                    y: 32132,
                    floor: -9,
                },
                false,
            )
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_katana_key_entry duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
