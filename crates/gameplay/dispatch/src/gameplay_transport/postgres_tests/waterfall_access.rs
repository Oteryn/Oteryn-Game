//! Waterfall's three source teleports run through the real STEP dispatcher.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn waterfall_access_steps_preserve_quest_state() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .name("waterfall-access".into())
        .stack_size(256 << 20)
        .spawn(|| {
            for (name, from, to) in [
                ("waterfall-cliff", (32966, 32626, -5), (32968, 32626, -5)),
                (
                    "waterfall-cave-entry",
                    (32968, 32629, -7),
                    (32968, 32631, -8),
                ),
                (
                    "waterfall-cave-exit",
                    (32967, 32630, -8),
                    (32971, 32620, -8),
                ),
            ] {
                let position = |(x, y, floor)| Position { x, y, floor };
                super::quest_world_callbacks::stateless_scenario(
                    name,
                    position(from),
                    position(to),
                )
                .map_err(|error| error.to_string())?;
            }
            Ok::<(), String>(())
        })?
        .join()
        .map_err(|_| "waterfall access scenario panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED waterfall_access Waterfall access duration={:?}",
        started.elapsed()
    );
    Ok(())
}
