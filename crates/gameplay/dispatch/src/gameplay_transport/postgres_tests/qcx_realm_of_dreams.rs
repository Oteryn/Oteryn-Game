//! TibiaWiki curid=61232&oldid=999253 specifies the five-minute wall reset.
use super::{TestResult, quest_world_callbacks};

#[test]
fn wall_restores_after_five_minutes() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            quest_world_callbacks::scenario(
                "m07/realm-trial",
                oteryn_gameplay_quests::quest::world_callbacks::Position {
                    x: 33080,
                    y: 32583,
                    floor: -3,
                },
                false,
            )
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Realm of Dreams worker panicked")?;
    eprintln!(
        "Realm of Dreams wall restoration duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result?;
    Ok(())
}
