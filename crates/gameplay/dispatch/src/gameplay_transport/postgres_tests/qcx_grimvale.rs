//! TibiaWiki Grimvale Quest/Spoiler, curid 75274, oldid 1103828.
use super::TestResult;

#[test]
fn full_moon_opens_at_server_save_on_the_twelfth() -> TestResult {
    moon_boundary(11, 12)
}

#[test]
fn full_moon_ends_at_server_save_on_the_fifteenth() -> TestResult {
    moon_boundary(15, 14)
}

fn moon_boundary(inactive_day: i32, active_day: i32) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qcx-grimvale", move || {
        super::worldchg_grimvale::scenario_with_moon_dates(false, inactive_day, active_day)
    });
    eprintln!(
        "QCX_GRIMVALE inactive_day={inactive_day} active_day={active_day} duration={:?}",
        started.elapsed()
    );
    result
}
