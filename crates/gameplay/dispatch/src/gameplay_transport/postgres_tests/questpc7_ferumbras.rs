//! Selected Ferumbras route and atomic world rewards over existing fenced owners.

use super::TestResult;
use super::questpc7_ferumbras_fixtures::PREDECESSOR_CASE;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
#[test]
fn questpc7_ferumbras_full_native_seals_and_reward_bundle() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-ferumbras", || {
            super::quest_world_callbacks_fixtures::scenario(
                "questpc7-ferumbras",
                Position {
                    x: 33275,
                    y: 32390,
                    floor: -8,
                },
                false,
            )
        });
    eprintln!(
        "QUESTPC7_FERUMBRAS POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn ferumbras_chest_claims_on_an_in_progress_predecessor_pin() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(PREDECESSOR_CASE, || {
        super::quest_world_callbacks_fixtures::scenario(
            PREDECESSOR_CASE,
            Position {
                x: 33275,
                y: 32390,
                floor: -8,
            },
            false,
        )
    })
}
