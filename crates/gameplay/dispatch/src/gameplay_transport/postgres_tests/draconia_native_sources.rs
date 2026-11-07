//! Exact native anchors omit startup UIDs; real USE/STEP must acquire them at boot.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn draconia_native_source_lever_30006() -> TestResult {
    scenario("draconia-lever-30006", 32792, 31595, -7)
}
#[test]
fn draconia_native_source_lever_30035() -> TestResult {
    scenario("draconia-lever-30035", 32792, 31579, -7)
}
#[test]
fn draconia_native_source_exit_35011() -> TestResult {
    scenario("draconia-exit-correct", 32805, 31587, -1)
}
#[test]
fn draconia_native_source_escape_35012() -> TestResult {
    scenario("draconia-escape-exact", 32815, 31599, -9)
}
fn scenario(name: &'static str, x: u16, y: u16, floor: i8) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || -> Result<(), String> {
            super::quest_world_callbacks::scenario(name, Position { x, y, floor }, false)
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "native Draconia source scenario panicked")??;
    println!(
        "POSTGRES_CASE_COMPLETED draconia_native_source {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}
