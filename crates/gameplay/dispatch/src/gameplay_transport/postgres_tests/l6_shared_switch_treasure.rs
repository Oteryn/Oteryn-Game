//! Personal expiry consequences follow the original live actor and real shared leases.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
#[test]
fn l6_switch_treasure_thalas_expiry_grants_one() -> TestResult {
    run("l6-tomb-switch-treasure-expired-12121")
}
#[test]
fn l6_switch_treasure_diprath_expiry_grants_two() -> TestResult {
    run("l6-tomb-switch-treasure-expired-12122")
}
#[test]
fn l6_switch_treasure_ashmunrah_expiry_grants_no_personal_progress() -> TestResult {
    run("l6-tomb-switch-treasure-expired-12123")
}
#[test]
fn l6_switch_treasure_repeat_cycles_do_not_repeat_personal_progress() -> TestResult {
    run("l6-tomb-switch-treasure-repeat-12121")
}
#[test]
fn l6_switch_treasure_future_lease_grants_no_personal_progress() -> TestResult {
    run("l6-tomb-switch-treasure-fresh-12121")
}
#[test]
fn l6_switch_treasure_absent_original_actor_grants_no_personal_progress() -> TestResult {
    run("l6-tomb-switch-treasure-offline-12122")
}
fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {name}");
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks_fixtures::scenario(
                name,
                Position {
                    x: 7,
                    y: 1,
                    floor: -7,
                },
                false,
            )
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "personal timer fixture panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
