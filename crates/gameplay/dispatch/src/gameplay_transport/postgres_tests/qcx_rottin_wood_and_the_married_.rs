use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn rottin_accepts_quest_and_requires_two_confirmations_for_disrepair() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("qcx72-rottin", || {
        super::quest_world_callbacks::scenario(
            "classic-rottin-disrepair",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!("QCX72_ROTTIN_PG_EXECUTED duration={:?}", started.elapsed());
    Ok(())
}
