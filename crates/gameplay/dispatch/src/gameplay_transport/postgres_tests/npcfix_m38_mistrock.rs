//! M38: reuse the existing native source program; a shared reward callback is unnecessary.
//! This proves the fenced dialogue step, not physical Mistrock access or the whole quest.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn npcfix_m38_mistrock_source_dialogue_commits_once_and_replays_postgres() -> TestResult {
    run_case("npcfix-m38-mistrock")
}

#[test]
fn npcfix_m38_mistrock_requires_current_premium_postgres() -> TestResult {
    run_case("npcfix-m38-mistrock-free")
}

fn run_case(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        super::quest_world_callbacks_fixtures::scenario(
            name,
            Position {
                x: 32641,
                y: 31436,
                floor: -7,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED NPCFIX_M38_MISTROCK case={name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}
