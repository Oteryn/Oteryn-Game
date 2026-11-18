use super::*;

#[test]
fn m01b_emperor_cookies_native_passage_entry_credits_once_and_replays() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e_fixtures::scenario_thread::run(
        "m01b-emperor-cookies-entry",
        move || {
            super::super::quest_world_callbacks_fixtures::scenario(
                "m01b/emperor-s-cookies/s1/id3800",
                Position {
                    x: 32605,
                    y: 31907,
                    floor: -3,
                },
                false,
            )
        },
    );
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_emperor_cookies_entry duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
